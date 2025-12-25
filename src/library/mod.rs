use core::error::Error;
use gio::prelude::FileExt;
use gtk::gio;
use rand::random_range;
use std::cmp::Ordering;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::thread;
use std::{fs, mem};
use tokio::sync::mpsc as tokio_mpsc;

pub mod album;
pub mod artist;
pub mod search;
pub mod song;

pub use album::Album;
pub use artist::Artist;
pub use song::{Song, SongInfo};

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::album::SortedAlbumSongs;
use crate::library::artist::SortedArtistAlbums;
use crate::library::search::query_score;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::tasks::{BoxedTask, Runner};
use crate::ui::{UI_TX, UpdateUI};
use crate::{CONFIG_DIR, visit_dirs};

// TODO: Support song/album ratings
// TODO: Implement song/album/artist search/filtering
// TODO: Efficient search/filter by tag, rating, titles, etc. Use SQL?

const FILE_SUPPORT: &[&str] = &[
    "flac", "m4a", "mp3", "aac", "ac3", "wav",
    // TODO: Ensure all listed formats work
    // Untested:
    "ape", "mpc", "ogg",
];

pub struct Library {
    pub songs: Songs,
    pub albums: Albums,
    pub artists: Artists,

    config: LibraryConfig,
    config_dir: String,

    tasks: Runner,
    player_tx: mpsc::Sender<PlayerRequest>,
    ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
    rx: mpsc::Receiver<LibraryRequest>,
}

#[derive(Default)]
pub struct LibraryConfig {
    pub directories: Vec<String>,
}

impl LibraryConfig {
    fn load() -> Self {
        // TODO: Load config from disk
        Self::default()
    }
}

// IDEA: Options to re-sort using different criteria,
// with the below functions respecting said option

pub type Songs = Vec<Arc<Mutex<Song>>>;
pub trait SortedSongs {
    fn find_song(&self, uri: &str, library_path_len: usize) -> Result<usize, usize>;
}
impl SortedSongs for Songs {
    #[inline]
    fn find_song(&self, uri: &str, library_uri_len: usize) -> Result<usize, usize> {
        self.binary_search_by(|song| {
            // Shortening the URI makes the lookup faster, however
            // files with identical relative paths will be ignored
            song.lock().unwrap().info().file_uri()[library_uri_len..].cmp(&uri[library_uri_len..])
        })
    }
}

pub type Albums = Vec<Arc<Mutex<Album>>>;
pub trait SortedAlbums {
    fn find_album(&self, info: &SongInfo) -> Result<usize, usize>;
}
impl SortedAlbums for Albums {
    #[inline]
    fn find_album(&self, info: &SongInfo) -> Result<usize, usize> {
        self.binary_search_by(|album| {
            let album = album.lock().unwrap();
            match album.artist.lock().unwrap().name.cmp(&info.album_artist) {
                Ordering::Equal => album.title.cmp(&info.album),
                ordering => ordering,
            }
        })
    }
}

pub type Artists = Vec<Arc<Mutex<Artist>>>;
pub trait SortedArtists {
    fn find_artist(&self, info: &SongInfo) -> Result<usize, usize>;
}
impl SortedArtists for Artists {
    #[inline]
    fn find_artist(&self, info: &SongInfo) -> Result<usize, usize> {
        self.binary_search_by(|artist| artist.lock().unwrap().name.cmp(&info.album_artist))
    }
}

pub static LIBRARY_TX: OnceLock<mpsc::Sender<LibraryRequest>> = OnceLock::new();
pub enum LibraryRequest {
    Rebuild,

    InitQueue,
    QueueFromPaths(Box<[String]>),
    PlayAllSongs,
    PlayAllAlbums,
    ShuffleAllAlbums,
    PlayAllArtists,
    ShuffleAllArtists,

    AddLibrary(Box<str>),
    EditLibrary(Box<(usize, String)>),
    RemoveLibrary(usize),
    SetLibraries(Box<[String]>),

    SetSongs(Songs),
    SetAlbums(Albums),
    SetArtists(Artists),

    RunTask(BoxedTask),
    Shutdown(mpsc::Sender<()>),
}

impl Library {
    #[must_use]
    pub fn init(
        player_tx: mpsc::Sender<PlayerRequest>,
        ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
    ) -> Library {
        let (tx, rx) = mpsc::channel();
        LIBRARY_TX.set(tx.clone()).map_err(|_| EXP_INIT).unwrap();

        Library {
            songs: vec![],
            albums: vec![],
            artists: vec![],

            config: LibraryConfig::load(),
            config_dir: CONFIG_DIR.get().expect(EXP_INIT).clone(),

            tasks: Runner::new(4),
            player_tx,
            ui_tx,
            rx,
        }
    }

    pub fn init_queue(&self) -> Result<(), Box<dyn Error>> {
        let mut args = std::env::args();
        args.next();

        // Start a queue from arguments, if they contain any supported files
        if let Some(queue) = self.songs_from_paths(&args.collect::<Box<[String]>>()) {
            self.player_tx.send(PlayerRequest::LoadQueue(queue))?;
            self.player_tx.send(PlayerRequest::SkipTo(0))?;
            return Ok(());
        }

        // Load the previous queue if file exists
        if let Ok(queue) = fs::read_to_string(self.config_dir.clone() + "queue") {
            'queue: {
                let mut lines = queue.lines();
                let Some(Ok(track)) = lines.next().map(str::parse) else {
                    break 'queue;
                };
                let Some(queue) =
                    self.songs_from_paths(&lines.map(String::from).collect::<Vec<_>>())
                else {
                    break 'queue;
                };

                self.player_tx.send(PlayerRequest::LoadQueue(queue))?;
                self.player_tx.send(PlayerRequest::SkipTo(track))?;
                return Ok(());
            }
        }

        // self.ui_tx.send(UpdateUI::FocusLibrary)?;
        // self.ui_tx.send(UpdateUI::OpenSheet(true))?;

        // TODO: Once the library pages work, uncomment the above instead
        self.player_tx.send(PlayerRequest::SetShuffle(true))?;
        self.play_all_songs()?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(false)))?;
        Ok(())
    }

    pub fn set_libraries(&mut self, dirs: &[String]) {
        self.config.directories = dirs.into();
        self.config.directories.sort();
        println!(
            "Library directories updated\nLibraries: {:?}",
            self.config.directories
        );
        self.ui_tx
            .send(UpdateUI::LibraryDirs(
                self.config.directories.clone().into(),
            ))
            .expect(EXP_RX);
    }

    pub fn add_library(&mut self, dir: String) {
        if self.config.directories.contains(&dir) || dir.is_empty() {
            return;
        }
        self.config.directories.push(dir);
        self.config.directories.sort();
        println!(
            "Added a new library\nLibraries: {:?}",
            self.config.directories
        );
        self.ui_tx
            .send(UpdateUI::LibraryDirs(
                self.config.directories.clone().into(),
            ))
            .expect(EXP_RX);
    }

    pub fn edit_library(&mut self, index: usize, dir: String) {
        if self.config.directories.contains(&dir) {
            return self.remove_library(index);
        }
        self.config.directories[index] = dir;
        self.config.directories.sort();
        println!("Edited a library\nLibraries: {:?}", self.config.directories);
        self.ui_tx
            .send(UpdateUI::LibraryDirs(
                self.config.directories.clone().into(),
            ))
            .expect(EXP_RX);
    }

    pub fn remove_library(&mut self, index: usize) {
        self.config.directories.remove(index);
        println!(
            "Removed a library\nLibraries: {:?}",
            self.config.directories
        );
        self.ui_tx
            .send(UpdateUI::LibraryDirs(
                self.config.directories.clone().into(),
            ))
            .expect(EXP_RX);
    }

    /// Serializes `songs` and writes the data to disk,
    /// so the library can be loaded faster next time
    ///
    /// Creates a file called `songs` in `self.config.config_dir`
    #[inline]
    fn serialize_songs(songs: &Songs) {
        let serialized = songs
            .iter()
            .map(|song| song.lock().unwrap().serlialize() + "\n")
            .collect::<String>()
            .trim()
            .to_string();
        match fs::create_dir_all(CONFIG_DIR.get().expect(EXP_INIT)).map(|()| {
            fs::write(
                CONFIG_DIR.get().expect(EXP_INIT).clone() + "songs",
                &serialized,
            )
        }) {
            Ok(Ok(())) => println!("Library song info has been successfully written to disk"),
            Ok(Err(e)) | Err(e) => eprintln!("Problems writing the library state to disk: {e}"),
        }
    }

    /// Reads the serialized song info from disk and returns them,
    /// so they can be assigned directly to `self.songs`
    ///
    /// Reads from a file called `songs` in `self.config.config_dir`
    #[must_use]
    fn deserialize_songs(&self) -> Songs {
        let data = fs::read_to_string(self.config_dir.clone() + "songs").unwrap_or_default();
        let data = data.split("\n\n");
        data.filter_map(|data| match Song::deserialize(data) {
            Ok(song) => Some(Arc::new(Mutex::new(song))),
            Err(_) => None,
        })
        .collect()
    }

    // Assigns `self.songs` by loading the serialized data (if any), then
    // inserting any new audio files found within the configured libraries
    pub fn discover_files(&mut self) -> Result<(), Box<dyn Error>> {
        if self.songs.is_empty() {
            self.songs = self.deserialize_songs();
        }

        let mut songs = Vec::new();
        mem::swap(&mut self.songs, &mut songs);
        let songs = Arc::new(Mutex::new(Some(songs)));

        // TODO: Check file modification times and update info/associations
        self.config.directories.iter().for_each(|library_path| {
            let to_relative = gio::File::for_path(library_path).uri().len();
            let _ = visit_dirs(Path::new(&library_path), &|f| {
                let file = gio::File::for_path(f.path().to_str().unwrap());
                if !Library::file_supported(&file.parse_name()) {
                    return;
                }

                let mut songs = songs.lock().unwrap();
                let songs = songs.as_mut().expect(EXP_INIT);
                let Err(index) = songs.find_song(&file.uri(), to_relative) else {
                    return;
                };

                let song = Arc::new(Mutex::new(Song::new(file)));
                self.tasks.run({
                    let song = Arc::clone(&song);
                    move || {
                        let _ = song.try_lock().map(|mut song| song.info().load_basic());
                    }
                });
                songs.insert(index, song);
            })
            .inspect_err(|e| println!("Error reading '{library_path}': {e}"));
        });
        let songs = songs.lock().unwrap().take().expect(EXP_INIT);

        let task_handle = thread::spawn({
            let songs = songs.clone();
            move || Library::create_associations(&songs).expect(EXP_RX)
        });
        self.tasks.run(move || task_handle.join().unwrap());

        // TODO: Check all files if they still exist, and detect if they were moved
        // 1: Go through all songs and check if they no longer exist on disk
        // 2: Move those to a list of missing songs (referred to as `old` from now on)
        // 3: Compare each old info against all songs in the library
        //   3.1: If a match is found, copy `….info().user()` to the new one
        //   Idea: Expand outwards from the old index when searching
        // 4: Remove the old songs from the library (on the main library thread)

        self.set_songs(songs);

        Ok(())
    }

    /// Creates connections between library `songs`, `albums`, and `artists`
    pub fn create_associations(songs: &Songs) -> Result<(), Box<dyn Error>> {
        let mut albums: Albums = Vec::new();
        let mut artists: Artists = Vec::new();
        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let ui_tx = UI_TX.get().expect(EXP_INIT);

        // TODO: Allow users to cancel, but serialize so it can continue later
        for (i, song) in songs.iter().enumerate() {
            let mut song_unwrapped = song.lock().unwrap();
            let mut info = song_unwrapped.info();
            let song_info = info.basic();

            let album_index = albums.find_album(song_info);
            let artist_index = artists.find_artist(song_info);

            match artist_index {
                Ok(artist_index) => match album_index {
                    Ok(album_index) => {
                        // Associate the current song with its album
                        let album_songs = &mut albums[album_index].lock().unwrap().songs;
                        let song_index = album_songs.find_album_song(song_info);
                        match song_index {
                            Err(song_index) | Ok(song_index) => {
                                album_songs.insert(song_index, Arc::clone(song));
                            }
                        }

                        song_unwrapped.album = Some(Arc::clone(&albums[album_index]));
                    }
                    Err(album_index) => {
                        // Create a new album entry for the artist,
                        // and associate the current song with it
                        let album = Arc::new(Mutex::new(Album {
                            title: song_info.album.clone(),
                            year: song_info.year,
                            songs: vec![Arc::clone(song)],
                            artist: Arc::clone(&artists[artist_index]),
                        }));
                        albums.insert(album_index, Arc::clone(&album));

                        // Associate the album with the artist
                        let artist_albums = &mut artists[artist_index].lock().unwrap().albums;
                        let album_index = artist_albums.find_artist_album(song_info);
                        match album_index {
                            Err(album_index) | Ok(album_index) => {
                                artist_albums.insert(album_index, Arc::clone(&album));
                            }
                        }

                        song_unwrapped.album = Some(Arc::clone(&album));
                    }
                },
                Err(artist_index) => {
                    // Create a new entry for the artist,
                    // and associate song/album/artist
                    let artist = Arc::new(Mutex::new(Artist {
                        name: song_info.album_artist.clone(),
                        albums: vec![],
                    }));
                    let album = Arc::new(Mutex::new(Album {
                        title: song_info.album.clone(),
                        year: song_info.year,
                        songs: vec![Arc::clone(song)],
                        artist: Arc::clone(&artist),
                    }));
                    artist.lock().unwrap().albums.push(Arc::clone(&album));
                    artists.insert(artist_index, artist);

                    // Add the album to `albums` as well
                    match album_index {
                        Err(album_index) | Ok(album_index) => {
                            albums.insert(album_index, Arc::clone(&album));
                        }
                    }

                    song_unwrapped.album = Some(album);
                }
            }

            ui_tx.send(UpdateUI::Progress(Some(i as f64 / songs.len() as f64)))?;
        }

        library_tx.send(LibraryRequest::SetAlbums(albums))?;
        library_tx.send(LibraryRequest::SetArtists(artists))?;

        ui_tx.send(UpdateUI::Progress(None))?;

        Ok(())
    }

    pub fn request_handler(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.rx.recv()? {
                LibraryRequest::Rebuild => self.discover_files()?,

                LibraryRequest::SetSongs(songs) => self.set_songs(songs),
                LibraryRequest::SetAlbums(albums) => self.set_albums(albums),
                LibraryRequest::SetArtists(artists) => self.set_artists(artists),

                LibraryRequest::InitQueue => self.init_queue()?,
                LibraryRequest::QueueFromPaths(paths) => self.play_from_paths(&paths)?,
                LibraryRequest::PlayAllSongs => self.play_all_songs()?,
                LibraryRequest::PlayAllAlbums => self.play_all_albums()?,
                LibraryRequest::ShuffleAllAlbums => self.shuffle_all_albums()?,
                LibraryRequest::PlayAllArtists => self.play_all_artists()?,
                LibraryRequest::ShuffleAllArtists => self.shuffle_all_artists()?,

                LibraryRequest::AddLibrary(dir) => self.add_library(dir.to_string()),
                LibraryRequest::EditLibrary(args) => self.edit_library(args.0, args.1),
                LibraryRequest::SetLibraries(dirs) => self.set_libraries(&dirs),
                LibraryRequest::RemoveLibrary(index) => self.remove_library(index),

                LibraryRequest::RunTask(task) => self.tasks.run(task),
                LibraryRequest::Shutdown(notify_done) => self.shutdown(&notify_done)?,
            }
        }
    }

    fn set_songs(&mut self, songs: Songs) {
        self.ui_tx
            .send(UpdateUI::LibrarySongs(songs.clone()))
            .expect(EXP_RX);
        self.songs = songs;
    }
    fn set_albums(&mut self, albums: Albums) {
        self.ui_tx
            .send(UpdateUI::LibraryAlbums(albums.clone()))
            .expect(EXP_RX);
        self.albums = albums;
    }
    fn set_artists(&mut self, artists: Artists) {
        self.ui_tx
            .send(UpdateUI::LibraryArtists(artists.clone()))
            .expect(EXP_RX);
        self.artists = artists;
    }

    pub fn shutdown(&mut self, notify_done: &mpsc::Sender<()>) -> Result<(), Box<dyn Error>> {
        let mut songs = Vec::new();
        mem::swap(&mut self.songs, &mut songs);
        self.tasks.run(move || Library::serialize_songs(&songs));
        self.tasks.shutdown();
        notify_done.send(()).expect(EXP_RX);
        Ok(())
    }

    pub fn play_all_songs(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_songs()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    pub fn play_all_albums(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_albums()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    pub fn shuffle_all_albums(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_albums_shuffled()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    pub fn play_all_artists(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_artists()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    pub fn shuffle_all_artists(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_artists_shuffled()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Returns `true` if the specified file has a supported extension,
    /// or `false` if it does not
    #[inline]
    #[must_use]
    pub fn file_supported(file: &str) -> bool {
        let Some(extension) = file.rsplit_once('.').map(|s| s.1.to_lowercase()) else {
            return false;
        };
        FILE_SUPPORT.iter().any(|&ext| extension == ext)
    }

    /// Returns a queue of all songs in the library
    #[must_use]
    pub fn all_songs(&self) -> Vec<QueueItem> {
        self.songs
            .iter()
            .map(|song| QueueItem::Song(Arc::clone(song)))
            .collect()
    }

    /// Returns a queue of all albums in the library,
    /// with sequential order of songs
    #[must_use]
    pub fn all_albums(&self) -> Vec<QueueItem> {
        let mut queue = Vec::<QueueItem>::with_capacity(self.songs.len());
        for album in &self.albums {
            for song in &album.lock().unwrap().songs {
                queue.push(QueueItem::Song(Arc::clone(song)));
            }
        }
        queue
    }

    /// Returns a queue of all albums in the library,
    /// with sequential order of songs, but randomly
    /// ordered albums
    #[must_use]
    pub fn all_albums_shuffled(&self) -> Vec<QueueItem> {
        let mut queue = Vec::with_capacity(self.songs.len());
        let mut shuffled: Vec<usize> = (0..self.albums.len()).collect();
        for i in 0..shuffled.len() {
            let rand_index = random_range(0..shuffled.len());
            shuffled.swap(i, rand_index);
        }
        for index in shuffled {
            for song in &self.albums[index].lock().unwrap().songs {
                queue.push(QueueItem::Song(Arc::clone(song)));
            }
        }
        queue
    }

    /// Returns a queue of all artists in the library,
    /// with albums and songs in sequential order
    #[must_use]
    pub fn all_artists(&self) -> Vec<QueueItem> {
        let mut queue = Vec::<QueueItem>::with_capacity(self.songs.len());
        for artist in &self.artists {
            for album in &artist.lock().unwrap().albums {
                for song in &album.lock().unwrap().songs {
                    queue.push(QueueItem::Song(Arc::clone(song)));
                }
            }
        }
        queue
    }

    /// Returns a queue of all artists in the library,
    /// with albums and songs in sequential order, but
    /// randomly ordered artists
    #[must_use]
    pub fn all_artists_shuffled(&self) -> Vec<QueueItem> {
        let mut queue = Vec::with_capacity(self.songs.len());
        let mut shuffled: Vec<usize> = (0..self.artists.len()).collect();
        for i in 0..shuffled.len() {
            let rand_index = random_range(0..shuffled.len());
            shuffled.swap(i, rand_index);
        }
        for index in shuffled {
            for album in &self.artists[index].lock().unwrap().albums {
                for song in &album.lock().unwrap().songs {
                    queue.push(QueueItem::Song(Arc::clone(song)));
                }
            }
        }
        queue
    }

    pub fn play_from_paths(&self, paths: &[String]) -> Result<(), mpsc::SendError<PlayerRequest>> {
        if let Some(queue) = self.songs_from_paths(paths) {
            self.player_tx.send(PlayerRequest::LoadQueue(queue))?;
            self.player_tx.send(PlayerRequest::SkipTo(0))?;
            self.player_tx.send(PlayerRequest::TogglePlay(Some(true)))?;
        }
        Ok(())
    }

    /// Returns a queue of all songs found within the specified `paths`,
    /// recursively. Returns `None` if no song files were found.
    #[must_use]
    pub fn songs_from_paths(&self, paths: &[String]) -> Option<Vec<QueueItem>> {
        let queue = Arc::new(Mutex::new(Some(Vec::new())));
        for file in paths {
            let path = Path::new(&file);
            if path.is_file() {
                // Add files from arguments to queue
                if !Library::file_supported(file) {
                    continue;
                }

                let song = self.queue_from_library_or_new(file);
                queue.lock().unwrap().as_mut().expect(EXP_INIT).push(song);
            } else if Path::exists(path) {
                // Add all files within directory arguments to queue
                let songs = Arc::new(Mutex::new(Some(Vec::new())));
                let _ = visit_dirs(path, &|file| {
                    let file = file.path();
                    let file = file.to_str().unwrap();
                    if !Library::file_supported(file) {
                        return;
                    }

                    let song = self.queue_from_library_or_new(file);

                    let mut songs = songs.lock().unwrap();
                    let songs = songs.as_mut().expect(EXP_INIT);
                    match songs.binary_search_by(|existing: &QueueItem| {
                        let to_relative = path.to_str().unwrap().len();
                        existing.as_song().info().file_path()[to_relative..]
                            .cmp(&song.as_song().info().file_path()[to_relative..])
                    }) {
                        Err(index) | Ok(index) => songs.insert(index, song),
                    }
                });

                (queue.lock().unwrap().as_mut().expect(EXP_INIT))
                    .extend(songs.lock().unwrap().take().expect(EXP_INIT));
            }
        }

        match queue.lock().unwrap().take() {
            Some(queue) if !queue.is_empty() => Some(queue),
            _ => None,
        }
    }

    fn queue_from_library_or_new(&self, file: &str) -> QueueItem {
        for dir in &self.config.directories {
            if file.starts_with(dir) {
                let dir = gio::File::for_path(dir);
                let file = gio::File::for_path(file);
                match self.songs.find_song(&file.uri(), dir.uri().len()) {
                    Ok(index) => return QueueItem::Song(Arc::clone(&self.songs[index])),
                    Err(_) => break,
                }
            }
        }
        QueueItem::Song(Arc::new(Mutex::new(Song::new_from_path(file))))
    }

    /// Returns a list of songs matching the given `query`,
    /// ordered by how well the query matches the song title
    fn query_song_titles(&self, query: &str) -> Songs {
        let mut matches = Vec::<(Arc<Mutex<Song>>, f64)>::new();
        for song in &self.songs {
            let score = query_score(query, &song.lock().unwrap().info().basic().title);
            let index = matches.binary_search_by(|item| score.total_cmp(&item.1));
            matches.insert(
                match index {
                    Err(index) | Ok(index) => index,
                },
                (Arc::clone(song), score),
            );
        }
        matches
            .iter()
            .filter_map(|song| match song.1 > 0.5 {
                true => Some(Arc::clone(&song.0)),
                false => None,
            })
            .collect()
    }
}
