use core::error::Error;
use gio::prelude::FileExt;
use gtk::gio;
use rand::random_range;
use std::cmp::Ordering;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, mpsc};
use std::{fs, mem};
use tokio::sync::mpsc as tokio_mpsc;

pub mod album;
pub mod artist;
pub mod config;
pub mod search;
pub mod song;

pub use album::Album;
pub use artist::Artist;
pub use song::{Song, SongInfo};

use crate::excuses::{EXP_INIT, EXP_RX, INIT_ERR};
use crate::library::album::SortedAlbumSongs;
use crate::library::artist::SortedArtistAlbums;
use crate::library::config::{FILE_SUPPORT, LibraryConfig};
use crate::player::PlayerRequest;
use crate::player::queue_item::QueueItem;
use crate::tasks::{BoxedTask, Runner};
use crate::ui::{UI_TX, UpdateUI};
use crate::{CONFIG_DIR, visit_dirs};

// TODO: Support song/album ratings
// TODO: Implement song/album/artist search/filtering
// TODO: Efficient search/filter by tag, rating, titles, etc. Use SQL?

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

// IDEA: Options to re-sort using different criteria,
// with the below functions respecting said option

pub type Songs = Vec<Arc<Mutex<Song>>>;
pub trait SortedSongs {
    fn find_song(&self, uri: &str, trim_start: usize) -> Result<usize, usize>;
}
impl SortedSongs for Songs {
    #[inline]
    fn find_song(&self, uri: &str, trim_start: usize) -> Result<usize, usize> {
        self.binary_search_by(|song| {
            song.lock().unwrap().info().file_uri()[trim_start..].cmp(&uri[trim_start..])
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
                Ordering::Equal => match album.year.cmp(&info.year) {
                    Ordering::Equal => album.title.cmp(&info.album),
                    ordering => ordering,
                },
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
    // TODO: Instead of requiring a query each time,
    // maybe store/update it separately?
    PlayAllSongs(String),
    PlayAllAlbums(String),
    ShuffleAllAlbums(String),
    PlayAllArtists(String),
    ShuffleAllArtists(String),

    PlayAlbum(usize),

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
    /// Returns a new `Library` instance and initializes `LIBRARY_TX`
    #[inline]
    #[must_use]
    pub fn init(
        player_tx: mpsc::Sender<PlayerRequest>,
        ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
    ) -> Library {
        let (tx, rx) = mpsc::channel();
        LIBRARY_TX.set(tx).map_err(|_| INIT_ERR).unwrap();

        Library {
            songs: Vec::new(),
            albums: Vec::new(),
            artists: Vec::new(),

            config: LibraryConfig::default(),
            config_dir: CONFIG_DIR.get().expect(EXP_INIT).clone(),

            tasks: Runner::new(4),
            player_tx,
            ui_tx,
            rx,
        }
    }

    /// Main loop for handling library requests
    #[inline]
    pub fn request_handler(&mut self) -> Result<(), Box<dyn Error>> {
        // FIX: Library requests blocked while building the library?
        // `AddLibrary` worked, but `RemoveLibrary` did not...
        loop {
            match self.rx.recv()? {
                LibraryRequest::Rebuild => self.discover_files()?,

                LibraryRequest::SetSongs(songs) => self.set_songs(songs),
                LibraryRequest::SetAlbums(albums) => self.set_albums(albums),
                LibraryRequest::SetArtists(artists) => self.set_artists(artists),

                LibraryRequest::InitQueue => self.init_queue()?,
                LibraryRequest::QueueFromPaths(paths) => self.play_from_paths(&paths)?,
                LibraryRequest::PlayAllSongs(query) => self.play_all_songs(&query)?,
                LibraryRequest::PlayAllAlbums(query) => self.play_all_albums(&query)?,
                LibraryRequest::ShuffleAllAlbums(query) => self.shuffle_all_albums(&query)?,
                LibraryRequest::PlayAllArtists(query) => self.play_all_artists(&query)?,
                LibraryRequest::ShuffleAllArtists(query) => self.shuffle_all_artists(&query)?,

                LibraryRequest::PlayAlbum(index) => {
                    self.play_album(&self.albums[index].lock().unwrap())?;
                }

                LibraryRequest::AddLibrary(dir) => self.config.add_library(dir.to_string()),
                LibraryRequest::EditLibrary(args) => self.config.edit_library(args.0, args.1),
                LibraryRequest::SetLibraries(dirs) => self.config.set_libraries(&dirs, &self.ui_tx),
                LibraryRequest::RemoveLibrary(index) => self.config.remove_library(index),

                LibraryRequest::RunTask(task) => self.tasks.run(task),
                LibraryRequest::Shutdown(notify_done) => self.shutdown(&notify_done)?,
            }
        }
    }

    /// Starts the initial player queue
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

        if self.songs.is_empty() {
            // Maybe open the settings page and focus on the directory options?
            // self.ui_tx.send(UpdateUI::FocusLibrary)?;
            self.ui_tx.send(UpdateUI::OpenSheet(true))?;
            return Ok(());
        }

        // self.player_tx.send(PlayerRequest::SetShuffle(true))?;
        self.play_all_songs("")?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(false)))?;
        Ok(())
    }

    // Assigns `self.songs` by loading the serialized data (if any), then
    // inserting any new audio files found within the configured libraries
    pub fn discover_files(&mut self) -> Result<(), Box<dyn Error>> {
        let songs = Arc::new(Mutex::new(Some(match self.songs.is_empty() {
            false => mem::take(&mut self.songs),
            true => self.deserialize_songs(),
        })));

        // TODO: Check file modification times and update info/associations
        for library_path in &self.config.directories {
            let _ = visit_dirs(Path::new(&library_path), &|f| {
                let file = gio::File::for_path(f.path().to_str().unwrap());
                if !file_supported(&file.parse_name()) {
                    return;
                }

                let mut songs = songs.lock().unwrap();
                // SAFETY: `songs` is initialized as `Some`
                let songs = unsafe { songs.as_mut().unwrap_unchecked() };
                let Err(index) = songs.find_song(&file.uri(), self.config.uri_opt()) else {
                    return;
                };

                let song = Arc::new(Mutex::new(Song::new(file)));
                songs.insert(index, song);
            })
            .inspect_err(|e| eprintln!("Error reading '{library_path}': {e}"));
        }
        let songs = songs.lock().unwrap().take().expect(EXP_INIT);

        self.tasks.run({
            let songs = songs.clone();
            move || Library::create_associations(&songs).expect(EXP_RX)
        });

        self.set_songs(songs);

        Ok(())
    }

    /// Returns a list of `songs` whose files still exist on disk
    pub fn filter_missing(songs: &Songs) -> Songs {
        // TODO: Filter songs which aren't in any configured paths
        // TODO: Keep missing songs stored somewhere?
        // In case a library is temporarily missing (for example on
        // removable storage), it would be better if the data for
        // those could be retained instead of completely forgotten
        songs
            .iter()
            .filter(|song| {
                song.lock()
                    .unwrap()
                    .info()
                    .file()
                    .path()
                    .is_some_and(|path| fs::exists(path).is_ok_and(|exists| exists))
            })
            .map(Arc::clone)
            .collect()
    }

    pub fn remove_duplicates(&mut self) {
        // TODO: Check all files if they still exist, and detect if they were moved
        // 1: Go through all songs and check if they no longer exist on disk
        // 2: Move those to a list of missing songs (referred to as `old` from now on)
        // 3: Compare each old info against all songs in the library
        //   3.1: If a match is found, copy `….info().user()` to the new one
        //   Idea: Expand outwards from the old index when searching
        // 4: Remove the old songs from the library (on the main library thread)
    }

    /// Creates connections between library `songs`, `albums`, and `artists`
    pub fn create_associations(songs: &Songs) -> Result<(), Box<dyn Error>> {
        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let ui_tx = UI_TX.get().expect(EXP_INIT);

        let songs = Library::filter_missing(songs);
        let mut albums = Vec::with_capacity(songs.len() / 16);
        let mut artists = Vec::with_capacity(songs.len() / 64);

        // Spawning more tasks than there are workers,
        // in case some finish sooner than others
        let chunk_size = songs.len() / 64;
        for i in 0..64 {
            let songs = songs[chunk_size * i..chunk_size * (i + 1)].to_vec();
            library_tx
                .send(LibraryRequest::RunTask(Box::new(move || {
                    for song in songs {
                        let _ = song.try_lock().map(|mut song| song.info().load_basic());
                    }
                })))
                .expect(EXP_RX);
        }

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
                        // SAFETY: `album_index` is guaranteed to be within bounds
                        let album_songs =
                            unsafe { &mut albums.get_unchecked(album_index).lock().unwrap().songs };
                        let song_index = album_songs.find_album_song(song_info);
                        match song_index {
                            Err(song_index) | Ok(song_index) => {
                                album_songs.insert(song_index, Arc::clone(song));
                            }
                        }

                        // SAFETY: `album_index` is guaranteed to be within bounds
                        song_unwrapped.album =
                            Some(Arc::clone(unsafe { albums.get_unchecked(album_index) }));
                    }
                    Err(album_index) => {
                        // Create a new album entry for the artist,
                        // and associate the current song with it
                        let album = Arc::new(Mutex::new(Album {
                            title: song_info.album.clone(),
                            year: song_info.year,
                            songs: vec![Arc::clone(song)],
                            // SAFETY: `artist_index` is guaranteed to be within bounds
                            artist: Arc::clone(unsafe { artists.get_unchecked(artist_index) }),
                        }));
                        albums.insert(album_index, Arc::clone(&album));

                        // Associate the album with the artist
                        // SAFETY: `artist_index` is guaranteed to be within bounds
                        let artist_albums = unsafe {
                            &mut artists.get_unchecked(artist_index).lock().unwrap().albums
                        };
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

            ui_tx.send(UpdateUI::Progress(Some(i as f64 / songs.len() as f64)))?;
        }

        library_tx.send(LibraryRequest::SetSongs(songs))?;
        library_tx.send(LibraryRequest::SetAlbums(albums))?;
        library_tx.send(LibraryRequest::SetArtists(artists))?;

        ui_tx.send(UpdateUI::Progress(None))?;

        Ok(())
    }

    /// Replaces `self.songs` with `songs`
    fn set_songs(&mut self, songs: Songs) {
        self.ui_tx
            .send(UpdateUI::LibrarySongs(songs.clone()))
            .expect(EXP_RX);
        self.songs = songs;
    }
    /// Replaces `self.albums` with `albums`
    fn set_albums(&mut self, albums: Albums) {
        self.ui_tx
            .send(UpdateUI::LibraryAlbums(albums.clone()))
            .expect(EXP_RX);
        self.albums = albums;
    }
    /// Replaces `self.artists` with `artists`
    fn set_artists(&mut self, artists: Artists) {
        self.ui_tx
            .send(UpdateUI::LibraryArtists(artists.clone()))
            .expect(EXP_RX);
        self.artists = artists;
    }

    /// Returns a queue of all songs in the library matching the given `query`
    #[must_use]
    pub fn all_songs(&self, query: &str) -> Vec<QueueItem> {
        if query.is_empty() {
            self.songs.iter().map(QueueItem::from_song).collect()
        } else {
            // TODO: Suppert filters? (e.g. rating > 3, tag: "calm" | "fun", etc)
            let song_results = search::query_items(&self.songs, query, |song, query| {
                search::query_score(query, &song.lock().unwrap().info().basic().title)
            });
            song_results.iter().map(QueueItem::from_song).collect()
        }
    }

    /// Starts a queue of all songs in the library matching the given `query`
    pub fn play_all_songs(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_songs(query)))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    pub fn play_album(&self, album: &MutexGuard<Album>) -> Result<(), Box<dyn Error>> {
        self.player_tx.send(PlayerRequest::LoadQueue(
            album.songs.iter().map(QueueItem::from_song).collect(),
        ))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Returns a queue of all albums in the library,
    /// with sequential order of songs
    #[must_use]
    pub fn all_albums(&self, query: &str) -> Vec<QueueItem> {
        let albums = if query.is_empty() {
            &self.albums
        } else {
            &search::query_items(&self.albums, query, |album, query| {
                search::query_score(query, &album.lock().unwrap().title)
            })
        };
        let mut queue = Vec::<QueueItem>::with_capacity(albums.len() * 8);
        for album in albums {
            for song in &album.lock().unwrap().songs {
                queue.push(QueueItem::Song(Arc::clone(song)));
            }
        }
        queue
    }

    /// Starts a queue of all albums in the library
    pub fn play_all_albums(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_albums(query)))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Returns a queue of all albums in the library,
    /// with sequential order of songs, but randomly
    /// ordered albums
    #[must_use]
    pub fn all_albums_shuffled(&self, query: &str) -> Vec<QueueItem> {
        let albums = if query.is_empty() {
            &self.albums
        } else {
            &search::query_items(&self.albums, query, |album, query| {
                search::query_score(query, &album.lock().unwrap().title)
            })
        };
        let mut queue = Vec::with_capacity(albums.len() * 8);
        let mut shuffled: Vec<usize> = (0..albums.len()).collect();
        for i in 0..shuffled.len() {
            let rand_index = random_range(0..shuffled.len());
            shuffled.swap(i, rand_index);
        }
        for index in shuffled {
            for song in &albums[index].lock().unwrap().songs {
                queue.push(QueueItem::Song(Arc::clone(song)));
            }
        }
        queue
    }

    /// Starts a randomly ordered queue of all albums in the library
    pub fn shuffle_all_albums(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_albums_shuffled(query)))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Returns a queue of all artists in the library,
    /// with albums and songs in sequential order
    #[must_use]
    pub fn all_artists(&self, query: &str) -> Vec<QueueItem> {
        let artists = if query.is_empty() {
            &self.artists
        } else {
            &search::query_items(&self.artists, query, |artist, query| {
                search::query_score(query, &artist.lock().unwrap().name)
            })
        };
        let mut queue = Vec::<QueueItem>::with_capacity(self.songs.len());
        for artist in artists {
            for album in &artist.lock().unwrap().albums {
                for song in &album.lock().unwrap().songs {
                    queue.push(QueueItem::Song(Arc::clone(song)));
                }
            }
        }
        queue
    }

    /// Starts a queue of all albums in the library
    pub fn play_all_artists(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_artists(query)))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Returns a queue of all artists in the library,
    /// with albums and songs in sequential order, but
    /// randomly ordered artists
    #[must_use]
    pub fn all_artists_shuffled(&self, query: &str) -> Vec<QueueItem> {
        let artists = if query.is_empty() {
            &self.artists
        } else {
            &search::query_items(&self.artists, query, |artist, query| {
                search::query_score(query, &artist.lock().unwrap().name)
            })
        };
        let mut queue = Vec::with_capacity(artists.len() * 16);
        let mut shuffled: Vec<usize> = (0..artists.len()).collect();
        for i in 0..shuffled.len() {
            let rand_index = random_range(0..shuffled.len());
            shuffled.swap(i, rand_index);
        }
        for index in shuffled {
            for album in &artists[index].lock().unwrap().albums {
                for song in &album.lock().unwrap().songs {
                    queue.push(QueueItem::Song(Arc::clone(song)));
                }
            }
        }
        queue
    }

    /// Starts a randomly ordered queue of all artists in the library
    pub fn shuffle_all_artists(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_artists_shuffled(query)))?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Starts a queue of all songs found within the specified `paths`,
    /// recursively. Does nothing if no song files were found.
    pub fn play_from_paths(&self, paths: &[String]) -> Result<(), Box<dyn Error>> {
        if let Some(queue) = self.songs_from_paths(paths) {
            self.player_tx.send(PlayerRequest::LoadQueue(queue))?;
            self.player_tx.send(PlayerRequest::SkipTo(0))?;
            self.player_tx.send(PlayerRequest::TogglePlay(Some(true)))?;
            self.ui_tx.send(UpdateUI::OpenSheet(false))?;
            self.ui_tx.send(UpdateUI::FocusPlaying)?;
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
            if file_supported(file) {
                // Add files from arguments to queue
                let song = self.queue_from_library_or_new(file);
                // SAFETY: `queue` is initialized as `Some`
                unsafe { queue.lock().unwrap().as_mut().unwrap_unchecked().push(song) };
            } else if path.is_dir() && Path::exists(path) {
                // Add all files within directory arguments to queue
                let songs = Arc::new(Mutex::new(Some(Vec::new())));
                let _ = visit_dirs(path, &|file| {
                    let file = file.path();
                    let file = file.to_str().unwrap();
                    if !file_supported(file) {
                        return;
                    }

                    let song = self.queue_from_library_or_new(file);

                    let mut songs = songs.lock().unwrap();
                    // SAFETY: `songs` is initialized as `Some`
                    let songs = unsafe { songs.as_mut().unwrap_unchecked() };
                    match songs.binary_search_by(|existing: &QueueItem| {
                        (existing.as_song().info().file_path())
                            .cmp(&song.as_song().info().file_path())
                    }) {
                        Err(index) | Ok(index) => songs.insert(index, song),
                    }
                });

                // SAFETY: `queue` and `songs` are initalized as `Some`
                unsafe {
                    (queue.lock().unwrap().as_mut().unwrap_unchecked())
                        .extend(songs.lock().unwrap().take().unwrap_unchecked());
                }
            } else if file == "Stopper" {
                // SAFETY: `queue` is initalized as `Some`
                unsafe {
                    (queue.lock().unwrap().as_mut().unwrap_unchecked()).push(QueueItem::Stopper);
                }
            }
        }

        match queue.lock().unwrap().take() {
            Some(queue) if !queue.is_empty() => Some(queue),
            _ => None,
        }
    }

    /// Attempts to locate the given `file` within the library and
    /// returns it, otherwise a new instance of `Song` is returned
    fn queue_from_library_or_new(&self, file: &str) -> QueueItem {
        for dir in &self.config.directories {
            if file.starts_with(dir) {
                let file = gio::File::for_path(file);
                match self.songs.find_song(&file.uri(), self.config.uri_opt()) {
                    Ok(index) => {
                        // SAFETY: `index` is `Ok`, therefore within bounds
                        return QueueItem::Song(Arc::clone(unsafe {
                            self.songs.get_unchecked(index)
                        }));
                    }
                    Err(_) => break,
                }
            }
        }
        QueueItem::Song(Arc::new(Mutex::new(Song::new_from_path(file))))
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
            .collect::<String>();
        match fs::create_dir_all(CONFIG_DIR.get().expect(EXP_INIT)).map(|()| {
            fs::write(
                CONFIG_DIR.get().expect(EXP_INIT).clone() + "songs",
                serialized.trim(),
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
        let Ok(data) = fs::read_to_string(self.config_dir.clone() + "songs") else {
            return Vec::with_capacity(512); // Estimate to reduce reallocations
        };
        data.split("\n\n")
            .filter_map(|data| match Song::deserialize(data) {
                Ok(song) => Some(Arc::new(Mutex::new(song))),
                Err(_) => None,
            })
            .collect()
    }

    /// Writes the configuration to disk and shuts down gracefully.
    /// Notifies the caller over the `notify_done` channel when done.
    pub fn shutdown(&mut self, notify_done: &mpsc::Sender<()>) -> Result<(), Box<dyn Error>> {
        let songs = mem::take(&mut self.songs);
        self.tasks.run(move || Library::serialize_songs(&songs));
        self.tasks.shutdown();
        notify_done.send(()).expect(EXP_RX);
        Ok(())
    }
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
