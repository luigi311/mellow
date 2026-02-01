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
use crate::library::album::{AlbumMutex, SortedAlbumSongs};
use crate::library::artist::{ArtistMutex, SortedArtistAlbums};
use crate::library::config::{FILE_SUPPORT, LibraryConfig};
use crate::library::song::SongInfoLoader;
use crate::player::PlayerRequest;
use crate::player::queue_item::QueueItem;
use crate::tasks::{BoxedTask, Runner};
use crate::ui::{UI_TX, UpdateUI};
use crate::{CONFIG_DIR, visit_dirs};

// TODO: Support album ratings
// TODO: Implement song/album/artist search/filtering
// TODO: Efficient search/filter by tag, rating, titles, etc

pub struct Library {
    pub songs: Songs,
    pub albums: Albums,
    pub artists: Artists,
    pub missing_songs: Songs,

    config: LibraryConfig,
    config_dir: String,

    tasks: Runner,
    player_tx: mpsc::Sender<PlayerRequest>,
    ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
    rx: mpsc::Receiver<LibraryRequest>,
}

pub trait ToQueue {
    fn to_queue(&self) -> Vec<QueueItem>;
}

pub trait ToShuffledQueue {
    fn to_shuffled_queue(&self) -> Vec<QueueItem>;
}

pub type Songs = Vec<Arc<Mutex<Song>>>;
pub trait SortedSongs {
    /// Returns `Ok(index)` if found, or `Err(index)` if not
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
impl ToQueue for Songs {
    fn to_queue(&self) -> Vec<QueueItem> {
        self.iter().map(QueueItem::from_song).collect()
    }
}

pub type Albums = Vec<Arc<Mutex<Album>>>;
pub trait SortedAlbums {
    /// Returns `Ok(index)` if found, or `Err(index)` if not
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
impl ToQueue for Albums {
    fn to_queue(&self) -> Vec<QueueItem> {
        let mut queue = Vec::<QueueItem>::with_capacity(self.len() * 8);
        for album in self {
            for song in &album.lock().unwrap().songs {
                queue.push(QueueItem::Song(Arc::clone(song)));
            }
        }
        queue
    }
}
impl ToShuffledQueue for Albums {
    fn to_shuffled_queue(&self) -> Vec<QueueItem> {
        let mut queue = Vec::with_capacity(self.len() * 8);
        let mut shuffled: Vec<usize> = (0..self.len()).collect();
        for i in 0..shuffled.len() {
            let rand_index = random_range(0..shuffled.len());
            shuffled.swap(i, rand_index);
        }
        for index in shuffled {
            for song in &self[index].lock().unwrap().songs {
                queue.push(QueueItem::Song(Arc::clone(song)));
            }
        }
        queue
    }
}

pub type Artists = Vec<Arc<Mutex<Artist>>>;
pub trait SortedArtists {
    /// Returns `Ok(index)` if found, or `Err(index)` if not
    fn find_artist(&self, info: &SongInfo) -> Result<usize, usize>;
}
impl SortedArtists for Artists {
    #[inline]
    fn find_artist(&self, info: &SongInfo) -> Result<usize, usize> {
        self.binary_search_by(|artist| artist.lock().unwrap().name.cmp(&info.album_artist))
    }
}
impl ToQueue for Artists {
    fn to_queue(&self) -> Vec<QueueItem> {
        let mut queue = Vec::<QueueItem>::with_capacity(self.len() * 16);
        for artist in self {
            for album in &artist.lock().unwrap().albums {
                for song in &album.lock().unwrap().songs {
                    queue.push(QueueItem::Song(Arc::clone(song)));
                }
            }
        }
        queue
    }
}
impl ToShuffledQueue for Artists {
    fn to_shuffled_queue(&self) -> Vec<QueueItem> {
        let mut queue = Vec::with_capacity(self.len() * 16);
        let mut shuffled: Vec<usize> = (0..self.len()).collect();
        for i in 0..shuffled.len() {
            let rand_index = random_range(0..shuffled.len());
            shuffled.swap(i, rand_index);
        }
        for index in shuffled {
            for album in &self[index].lock().unwrap().albums {
                for song in &album.lock().unwrap().songs {
                    queue.push(QueueItem::Song(Arc::clone(song)));
                }
            }
        }
        queue
    }
}

pub static LIBRARY_TX: OnceLock<mpsc::Sender<LibraryRequest>> = OnceLock::new();
pub enum LibraryRequest {
    Rebuild,

    InitQueue,
    QueueFromPaths(Box<[String]>),

    // TODO: Filter and start the queue directly from the UI instead
    // (using the `ToQueue`/`ToShuffledQueue` traits)
    PlayAllSongs(String),
    PlayAllAlbums(String),
    ShuffleAllAlbums(String),
    PlayAllArtists(String),
    ShuffleAllArtists(String),

    PlayAlbum(AlbumMutex),
    PlayArtist(ArtistMutex),
    ShuffleArtist(ArtistMutex),

    AddLibrary(Box<str>),
    EditLibrary(Box<(usize, String)>),
    RemoveLibrary(usize),
    SetLibraries(Box<[String]>),

    SetSongs(Songs),
    SetAlbums(Albums),
    SetArtists(Artists),
    SetMissingSongs(Songs),

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
            missing_songs: Vec::new(),

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
                LibraryRequest::SetMissingSongs(songs) => self.set_missing_songs(songs),

                LibraryRequest::InitQueue => self.init_queue()?,
                LibraryRequest::QueueFromPaths(paths) => self.play_from_paths(&paths)?,
                LibraryRequest::PlayAllSongs(query) => self.play_all_songs(&query)?,
                LibraryRequest::PlayAllAlbums(query) => self.play_all_albums(&query)?,
                LibraryRequest::ShuffleAllAlbums(query) => self.shuffle_all_albums(&query)?,
                LibraryRequest::PlayAllArtists(query) => self.play_all_artists(&query)?,
                LibraryRequest::ShuffleAllArtists(query) => self.shuffle_all_artists(&query)?,

                LibraryRequest::PlayAlbum(album) => self.play_album(&album.lock().unwrap())?,
                LibraryRequest::PlayArtist(artist) => self.play_artist(&artist.lock().unwrap())?,
                LibraryRequest::ShuffleArtist(artist) => {
                    self.shuffle_artist_albums(&artist.lock().unwrap())?;
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

    /// Assigns `self.songs` by loading the serialized data (if any), then
    /// inserting any new audio files found within the configured libraries
    pub fn discover_files(&mut self) -> Result<(), Box<dyn Error>> {
        let mut songs = match self.songs.is_empty() {
            false => mem::take(&mut self.songs),
            true => self.deserialize_songs(),
        };

        for library_path in &self.config.directories {
            let _ = visit_dirs(Path::new(&library_path), &mut |f| {
                let file = gio::File::for_path(f.path().to_str().unwrap());
                if !file_supported(&file.parse_name()) {
                    return;
                }

                // Add song to library if it is not already there
                if let Err(index) = songs.find_song(&file.uri(), self.config.uri_opt()) {
                    songs.insert(index, Arc::new(Mutex::new(Song::new(file))));
                };
            })
            .inspect_err(|e| eprintln!("Error reading '{library_path}': {e}"));
        }
        self.songs = songs.clone();

        self.tasks.run({
            let missing_songs = self.missing_songs.clone();
            let config = self.config.clone();
            move || Library::create_connections(songs, missing_songs, config).expect(EXP_RX)
        });

        Ok(())
    }

    /// Creates connections between library `songs`, `albums`, and `artists`
    pub fn create_connections(
        mut songs: Songs,
        mut missing: Songs,
        config: LibraryConfig,
    ) -> Result<(), Box<dyn Error>> {
        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let ui_tx = UI_TX.get().expect(EXP_INIT);

        let possibly_moved = Library::validate_songs(&mut songs, &mut missing, &config);
        library_tx.send(LibraryRequest::SetMissingSongs(missing))?;

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

        Library::merge_moved_entries(&songs, possibly_moved, &config);

        // TODO: Allow users to cancel, but serialize so it can continue later
        let mut progress = 0.0;
        let step_size = 1.0 / songs.len() as f64;
        for song in &songs {
            let mut song_locked = song.lock().unwrap();
            let mut info = song_locked.info();
            let song_info = info.basic();

            let album_index = albums.find_album(song_info);
            let artist_index = artists.find_artist(song_info);

            match artist_index {
                Ok(artist_index) => match album_index {
                    Ok(album_index) => {
                        // SAFETY: `album_index` is `Ok`, therefore within bounds
                        let album = unsafe { albums.get_unchecked(album_index) };

                        // Add the song to the album songs
                        let album_songs = &mut album.lock().unwrap().songs;
                        let song_index = album_songs.find_album_song(song_info);
                        match song_index {
                            Err(song_index) | Ok(song_index) => {
                                album_songs.insert(song_index, Arc::clone(song));
                            }
                        }

                        // Associate the song with its album
                        song_locked.album = Some(Arc::clone(album));
                    }
                    Err(album_index) => {
                        // SAFETY: `artist_index` is `Ok`, therefore within bounds
                        let artist = unsafe { artists.get_unchecked(artist_index) };
                        let album = Arc::new(Mutex::new(Album {
                            title: song_info.album.clone(),
                            year: song_info.year,
                            songs: vec![Arc::clone(song)],
                            artist: Arc::clone(artist),
                        }));

                        // Add the album to `albums` and the artist's albums
                        albums.insert(album_index, Arc::clone(&album));
                        let artist_albums = &mut artist.lock().unwrap().albums;
                        let album_index = artist_albums.find_artist_album(song_info);
                        match album_index {
                            Err(album_index) | Ok(album_index) => {
                                artist_albums.insert(album_index, Arc::clone(&album));
                            }
                        }

                        // Associate the song with its album
                        song_locked.album = Some(Arc::clone(&album));
                    }
                },
                Err(artist_index) => {
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

                    // Add the album to `albums` and the artist's albums
                    artist.lock().unwrap().albums.push(Arc::clone(&album));
                    match album_index {
                        Err(album_index) | Ok(album_index) => {
                            albums.insert(album_index, Arc::clone(&album));
                        }
                    }

                    // Add the artist entry
                    artists.insert(artist_index, artist);

                    // Associate the song with its album
                    song_locked.album = Some(album);
                }
            }
            drop(song_locked);

            progress += step_size;
            let _ = ui_tx.send(UpdateUI::Progress(Some(progress)));
        }

        library_tx.send(LibraryRequest::SetArtists(artists))?;
        library_tx.send(LibraryRequest::SetAlbums(albums))?;
        library_tx.send(LibraryRequest::SetSongs(songs))?;

        ui_tx.send(UpdateUI::Progress(None))?;

        Ok(())
    }

    /// Ensures validity of the provided `songs`:
    /// - Sorts `songs` and resolves duplicate entries
    /// - Moves missing files from `songs` into `missing_songs`
    /// - Removes and returns a list of `songs` whose files may
    ///   have been moved on disk
    pub fn validate_songs(songs: &mut Songs, missing: &mut Songs, config: &LibraryConfig) -> Songs {
        let mut old_songs = mem::replace(songs, Vec::with_capacity(songs.len()));
        old_songs.append(missing);
        let mut possibly_moved = Vec::new();
        'iter: for song in old_songs {
            let mut song_locked = song.lock().unwrap();
            let mut info = song_locked.info();
            let missing_libraries = config.directories.iter().filter_map(|dir| {
                match fs::exists(dir).unwrap_or(false) {
                    false => Some(gio::File::for_path(dir).uri()),
                    true => None,
                }
            });
            match songs.find_song(&info.file_uri(), config.uri_opt()) {
                // Valid song entry
                Err(index)
                    if info
                        .file()
                        .path()
                        .is_some_and(|path| fs::exists(path).is_ok_and(|exists| exists)) =>
                {
                    for dir in &config.directories {
                        // Filter songs outside of `config.directories`
                        if !info.file_path().starts_with(dir) {
                            continue;
                        }
                        if info.file_modification_time() != info.known_modification_time() {
                            if info.known_modification_time() != 0 {
                                // Only print if it isn't a new file
                                println!("{}: reloading info", info.filename());
                            }
                            info.unload_basic();
                        }
                        drop(song_locked);
                        songs.insert(index, song);
                        continue 'iter;
                    }
                    // IDEA: To disable libraries, move `songs` into `disabled_songs`
                    drop(song_locked);
                    drop(song);
                }
                // Missing file
                Err(_) => {
                    let uri = &info.file_uri();
                    match missing.find_song(uri, config.uri_opt()) {
                        // New missing song entry
                        Err(index) => {
                            for dir in missing_libraries {
                                // Only remember missing files if they are within
                                // a library directory which is currently missing
                                // (otherwise, they were either moved or removed)
                                if uri[config.uri_opt()..].starts_with(&dir[config.uri_opt()..]) {
                                    println!(
                                        "Remembering {} because its library is missing",
                                        info.filename()
                                    );
                                    drop(song_locked);
                                    missing.insert(index, song);
                                    continue 'iter;
                                }
                            }
                            drop(song_locked);
                            possibly_moved.push(song);
                        }
                        // Duplicate missing song entry
                        Ok(index) => {
                            info.user_mut()
                                .merge_with(missing[index].lock().unwrap().info().user());
                            drop(song_locked);
                            drop(song);
                        }
                    }
                }
                // Duplicate entry
                Ok(index) => {
                    println!("Resolving duplicate entry: {}", info.filename());
                    info.user_mut()
                        .merge_with(songs[index].lock().unwrap().info().user());
                    drop(song_locked);
                    drop(song);
                }
            }
        }
        possibly_moved
    }

    /// Attempts to locate missing files if they were moved and merges
    /// them with the existing song entries so their info is preserved
    pub fn merge_moved_entries(songs: &Songs, possibly_moved: Songs, config: &LibraryConfig) {
        if possibly_moved.is_empty() {
            return;
        }

        let ui_tx = UI_TX.get().expect(EXP_INIT);
        let mut progress = 0.0;
        let step_size = 1.0 / possibly_moved.len() as f64;

        for missing in possibly_moved {
            let mut missing_locked = missing.lock().unwrap();
            let old_info = missing_locked.info();

            // Optimization: start with an initial guess and expand outwards
            let guess = match songs.find_song(&old_info.file_uri(), config.uri_opt()) {
                Err(index) | Ok(index) => index,
            };
            let (mut left, mut right) = (songs[0..guess].iter(), songs[guess..].iter());
            fn merge_if_matching(info: &mut SongInfoLoader, cmp_info: &SongInfoLoader) -> bool {
                if cmp_info.inspect_basic() == Some(info.basic()) {
                    // Copy the user-assigned song info to the new entry
                    println!("Found moved file: {}", cmp_info.filename());
                    info.user_mut().merge_with(cmp_info.user());
                    return true;
                }
                false
            }
            loop {
                let (left, right) = (left.next_back(), right.next());
                if right.is_some_and(|song| {
                    merge_if_matching(&mut song.lock().unwrap().info(), &old_info)
                }) || left.is_some_and(|song| {
                    merge_if_matching(&mut song.lock().unwrap().info(), &old_info)
                }) || (left.is_none() && right.is_none())
                {
                    break;
                }
            }
            progress += step_size;
            let _ = ui_tx.send(UpdateUI::Progress(Some(progress)));
        }
        ui_tx.send(UpdateUI::Progress(None)).expect(EXP_RX);
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
    /// Replaces `self.missing_songs` with `missing_songs`
    fn set_missing_songs(&mut self, missing_songs: Songs) {
        self.missing_songs = missing_songs;
    }

    /// Starts the initial player queue
    pub fn init_queue(&self) -> Result<(), Box<dyn Error>> {
        let mut args = std::env::args();
        args.next();

        // Start a queue from arguments, if they contain any supported files
        if args.len() > 0
            && let Some(queue) = self.songs_from_paths(&args.collect::<Box<[String]>>())
        {
            self.player_tx.send(PlayerRequest::LoadQueue((queue, 0)))?;
            return Ok(());
        }

        // Load the previous queue if file exists
        if let Ok(queue) = fs::read_to_string(self.config_dir.clone() + "queue")
            && let mut lines = queue.lines()
            && let Some(Ok(track)) = lines.next().map(str::parse)
            && let Some(queue) = self.songs_from_paths(&lines.map(String::from).collect::<Vec<_>>())
        {
            self.player_tx
                .send(PlayerRequest::LoadQueue((queue, track)))?;
            return Ok(());
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

    /// Returns a queue of all songs in the library matching the given `query`
    #[must_use]
    pub fn all_songs(&self, query: &str) -> Vec<QueueItem> {
        // TODO: Suppert filters? (e.g. rating > 3, tag: "calm" | "fun", etc)
        search::query_items(&self.songs, query, |song, query| {
            search::query_score(query, &song.lock().unwrap().info().basic().title)
        })
        .to_queue()
    }

    /// Starts a queue of all songs in the library matching the given `query`
    pub fn play_all_songs(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue((self.all_songs(query), 0)))?;
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
        search::query_items(&self.albums, query, |album, query| {
            search::query_score(query, &album.lock().unwrap().title)
        })
        .to_queue()
    }

    /// Starts a queue of all albums in the library
    pub fn play_all_albums(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue((self.all_albums(query), 0)))?;
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
        search::query_items(&self.albums, query, |album, query| {
            search::query_score(query, &album.lock().unwrap().title)
        })
        .to_shuffled_queue()
    }

    /// Starts a randomly ordered queue of all albums in the library
    pub fn shuffle_all_albums(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx.send(PlayerRequest::LoadQueue((
            self.all_albums_shuffled(query),
            0,
        )))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Starts a queue using songs from the given album
    pub fn play_album(&self, album: &MutexGuard<Album>) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue((album.songs.to_queue(), 0)))?;
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
        search::query_items(&self.artists, query, |artist, query| {
            search::query_score(query, &artist.lock().unwrap().name)
        })
        .to_queue()
    }

    /// Starts a queue of all albums in the library
    pub fn play_all_artists(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue((self.all_artists(query), 0)))?;
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
        search::query_items(&self.artists, query, |artist, query| {
            search::query_score(query, &artist.lock().unwrap().name)
        })
        .to_shuffled_queue()
    }

    /// Starts a randomly ordered queue of all artists in the library
    pub fn shuffle_all_artists(&self, query: &str) -> Result<(), Box<dyn Error>> {
        self.player_tx.send(PlayerRequest::LoadQueue((
            self.all_artists_shuffled(query),
            0,
        )))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Starts a queue using songs by the given artist
    pub fn play_artist(&self, artist: &MutexGuard<Artist>) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue((artist.to_queue(), 0)))?;
        self.player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Starts a queue of randomly ordered albums by the given artist
    pub fn shuffle_artist_albums(&self, artist: &MutexGuard<Artist>) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue((artist.to_shuffled_queue(), 0)))?;
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
            self.player_tx.send(PlayerRequest::LoadQueue((queue, 0)))?;
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
        let mut queue = Vec::with_capacity(16.max(paths.len()));
        for file in paths {
            if file_supported(file) {
                // Add files from arguments to queue
                queue.push(self.queue_from_library_or_new(file));
            } else if file == "Stopper" {
                queue.push(QueueItem::Stopper);
            } else if let path = Path::new(&file)
                && path.is_dir()
                && Path::exists(path)
            {
                // Add all files within directory arguments to queue
                let mut songs = Vec::with_capacity(16);
                let _ = visit_dirs(path, &mut |file| {
                    let file = file.path();
                    let file = file.to_str().unwrap();
                    if !file_supported(file) {
                        return;
                    }

                    let song = self.queue_from_library_or_new(file);
                    match songs.binary_search_by(|existing: &QueueItem| {
                        (existing.as_song().info().file_path())
                            .cmp(&song.as_song().info().file_path())
                    }) {
                        Err(index) | Ok(index) => songs.insert(index, song),
                    }
                });
                queue.extend(songs);
            }
        }

        match queue.is_empty() {
            false => Some(queue),
            true => None,
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
        let mut songs = mem::take(&mut self.songs);
        for missing_song in mem::take(&mut self.missing_songs) {
            // Re-insert missing songs so their info is kept
            let Err(index) = songs.find_song(
                &missing_song.lock().unwrap().info().file_uri(),
                self.config.uri_opt(),
            ) else {
                continue;
            };
            songs.insert(index, missing_song);
        }
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
