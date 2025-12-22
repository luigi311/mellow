use core::error::Error;
use gio::prelude::FileExt;
use gtk::{gio, glib};
use rand::random_range;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::{fs, io, mem};
use tokio::sync::mpsc as tokio_mpsc;

pub mod album;
pub mod artist;
pub mod song;

pub use album::Album;
pub use artist::Artist;
pub use song::{Song, SongInfo};

use crate::excuses::{EXP_INIT, EXP_RX, INIT_ERR};
use crate::library::album::SortedAlbumSongs;
use crate::library::artist::SortedArtistAlbums;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::tasks::Runner;
use crate::ui::UpdateUI;
use crate::visit_dirs;

// TODO: Support song/album ratings
// TODO: Implement song/album/artist search/filtering
// TODO: Efficient search/filter by tag, rating, titles, etc. Use SQL?

pub static CONFIG_DIR: OnceLock<String> = OnceLock::new();

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
    player_tx: mpsc::SyncSender<PlayerRequest>,
    ui_tx: tokio_mpsc::Sender<UpdateUI>,
    // tx: mpsc::SyncSender<LibraryRequest>,
    rx: mpsc::Receiver<LibraryRequest>,
}

pub struct LibraryConfig {
    pub directories: Vec<String>,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        LibraryConfig {
            directories: [
                glib::user_special_dir(glib::UserDirectory::Music).map_or_else(
                    || [glib::home_dir().to_str().unwrap(), "/Music/"].concat(),
                    |dir| dir.to_str().unwrap().to_string(),
                ),
            ]
            .into(),
        }
    }
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
        // IDEA: Improve `albums` sorting: artist/year/title or artist/title
        self.binary_search_by(|album| album.lock().unwrap().title.cmp(&info.album))
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

pub enum LibraryRequest {
    InitQueue,
    QueueFromPaths(Box<[String]>),
    PlayAllSongs,
    PlayAllAlbums,
    ShuffleAllAlbums,
    PlayAllArtists,
    ShuffleAllArtists,
    Rebuild,
    AddLibrary(Box<str>),
    EditLibrary(Box<(usize, String)>),
    RemoveLibrary(usize),
}

impl Library {
    #[must_use]
    pub fn init(
        player_tx: mpsc::SyncSender<PlayerRequest>,
        ui_tx: tokio_mpsc::Sender<UpdateUI>,
    ) -> (Library, mpsc::SyncSender<LibraryRequest>) {
        CONFIG_DIR
            .set(glib::user_config_dir().to_str().unwrap().to_string() + "/mellow/")
            .expect(INIT_ERR);

        let (tx, rx) = mpsc::sync_channel(4);
        let library = Library {
            songs: vec![],
            albums: vec![],
            artists: vec![],

            config: LibraryConfig::load(),
            config_dir: CONFIG_DIR.get().expect(EXP_INIT).clone(),

            tasks: Runner::new_thread_pool(4),
            player_tx,
            ui_tx,
            // tx: tx.clone(),
            rx,
        };

        tx.send(LibraryRequest::Rebuild).expect(EXP_RX);
        tx.send(LibraryRequest::InitQueue).expect(EXP_RX);

        (library, tx)
    }

    pub async fn init_queue(&self) -> Result<(), Box<dyn Error>> {
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
                let Some(Ok(track)) = lines.next().map(|line| line.parse()) else {
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

        // self.ui_tx.send(UpdateUI::FocusLibrary).await?;
        // self.ui_tx.send(UpdateUI::OpenSheet(true)).await?;

        // TODO: Once the library pages work, uncomment the above instead
        self.player_tx.send(PlayerRequest::SetShuffle(true))?;
        self.play_all_songs().await?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(false)))?;
        Ok(())
    }

    pub async fn add_library(&mut self, dir: String) {
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
            .await
            .expect(EXP_RX);
    }

    pub async fn edit_library(&mut self, index: usize, dir: String) {
        if self.config.directories.contains(&dir) {
            return self.remove_library(index).await;
        }
        self.config.directories[index] = dir;
        self.config.directories.sort();
        println!("Edited a library\nLibraries: {:?}", self.config.directories);
        self.ui_tx
            .send(UpdateUI::LibraryDirs(
                self.config.directories.clone().into(),
            ))
            .await
            .expect(EXP_RX);
    }

    pub async fn remove_library(&mut self, index: usize) {
        self.config.directories.remove(index);
        println!(
            "Removed a library\nLibraries: {:?}",
            self.config.directories
        );
        self.ui_tx
            .send(UpdateUI::LibraryDirs(
                self.config.directories.clone().into(),
            ))
            .await
            .expect(EXP_RX);
    }

    /// Serializes `songs` and writes the data to disk,
    /// so the library can be loaded faster next time
    ///
    /// Creates a file called `songs` in `self.config.config_dir`
    #[inline]
    fn serialize_songs(songs: &Songs) -> io::Result<()> {
        let serialized = songs
            .iter()
            .map(|song| song.lock().unwrap().serlialize() + "\n")
            .collect::<String>()
            .trim()
            .to_string();
        fs::create_dir_all(&CONFIG_DIR.get().expect(EXP_INIT))?;
        fs::write(
            CONFIG_DIR.get().expect(EXP_INIT).clone() + "songs",
            &serialized,
        )
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

    /// Creates connections between library `songs`, `albums`, and `artists`
    #[allow(clippy::await_holding_lock)] // False-positive warning
    pub async fn discover_files(&mut self) -> Result<(), Box<dyn Error>> {
        if self.songs.is_empty() {
            self.songs = self.deserialize_songs();
        }

        // TODO: Check file modification times and update info/associations
        // TODO: Remove missing songs
        // IDEA: Find moved songs

        let mut songs = Vec::new();
        mem::swap(&mut self.songs, &mut songs);
        let songs = Arc::new(Mutex::new(Some(songs)));

        self.config.directories.iter().for_each(|library_path| {
            let to_relative = gio::File::for_path(library_path).uri().len();
            let _ = visit_dirs(Path::new(&library_path), &|f| {
                let file = gio::File::for_path(f.path().to_str().unwrap());
                if !Library::file_supported(&file.parse_name()) {
                    return;
                }

                let mut songs = songs.lock().unwrap();
                let songs = songs.as_mut().expect(EXP_INIT);
                let index = songs.find_song(&file.uri(), to_relative);
                let Err(index) = index else {
                    return;
                };

                let song = Arc::new(Mutex::new(Song::new(file)));
                songs.insert(index, song);
            })
            .inspect_err(|e| println!("Error reading '{library_path}': {e}"));
        });
        let songs = songs.lock().unwrap().take().expect(EXP_INIT);

        self.ui_tx
            .send(UpdateUI::LibrarySongs(songs.clone()))
            .await?;
        self.songs = songs;

        self.create_associations().await
    }

    pub async fn create_associations(&mut self) -> Result<(), Box<dyn Error>> {
        // IDEA: Do the rest in a background thread
        // Add three new `LibraryRequest`s: `SetProgress`/`SetAlbums`/`SetArtists`,
        // then all it would take would be to clone the `songs` again and pass them
        // to the worker thread

        let mut changed = false;
        let mut albums: Albums = Vec::new();
        let mut artists: Artists = Vec::new();

        // TODO: Allow users to cancel, but serialize so it can continue later
        const PROGRESS_BAR_STEPS: usize = 270; // IDEA: Use window width?
        let progress_freq = self.songs.len() / PROGRESS_BAR_STEPS + 1;
        for (i, song) in self.songs.iter().enumerate() {
            let mut song_unwrapped = song.lock().unwrap();
            let mut info = song_unwrapped.info();
            let song_info = info.basic_and(|| changed = true);

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
            drop(song_unwrapped);

            if i % progress_freq == 0 {
                let progress = Some(i as f64 / self.songs.len() as f64);
                self.ui_tx.send(UpdateUI::Progress(progress)).await?;
            }
        }

        self.ui_tx
            .send(UpdateUI::LibraryAlbums(albums.clone()))
            .await?;
        self.albums = albums;

        self.ui_tx
            .send(UpdateUI::LibraryArtists(artists.clone()))
            .await?;
        self.artists = artists;

        if changed {
            // self.serialize_songs().unwrap();
            // This is pretty fast anyway, so maybe not worth it...
            self.tasks.run({
                let songs = self.songs.clone();
                move || Library::serialize_songs(&songs).unwrap()
            });
        }

        self.ui_tx.send(UpdateUI::Progress(None)).await?;
        Ok(())
    }

    pub async fn request_handler(&mut self) -> Result<(), Box<dyn Error>> {
        self.ui_tx
            .send(UpdateUI::LibraryDirs(
                self.config.directories.clone().into(),
            ))
            .await?;

        loop {
            match self.rx.recv()? {
                LibraryRequest::InitQueue => self.init_queue().await?,
                LibraryRequest::QueueFromPaths(paths) => self.play_from_paths(&paths)?,
                LibraryRequest::PlayAllSongs => self.play_all_songs().await?,
                LibraryRequest::PlayAllAlbums => self.play_all_albums().await?,
                LibraryRequest::ShuffleAllAlbums => self.shuffle_all_albums().await?,
                LibraryRequest::PlayAllArtists => self.play_all_artists().await?,
                LibraryRequest::ShuffleAllArtists => self.shuffle_all_artists().await?,
                LibraryRequest::Rebuild => self.discover_files().await?,
                LibraryRequest::AddLibrary(dir) => self.add_library(dir.to_string()).await,
                LibraryRequest::EditLibrary(args) => self.edit_library(args.0, args.1).await,
                LibraryRequest::RemoveLibrary(index) => self.remove_library(index).await,
            }
        }
    }

    pub async fn play_all_songs(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_songs()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
        Ok(())
    }

    pub async fn play_all_albums(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_albums()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
        Ok(())
    }

    pub async fn shuffle_all_albums(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_albums_shuffled()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
        Ok(())
    }

    pub async fn play_all_artists(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_artists()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
        Ok(())
    }

    pub async fn shuffle_all_artists(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_artists_shuffled()))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
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
}
