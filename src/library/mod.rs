use core::cmp::Ordering;
use core::sync::atomic::{self, AtomicBool};
use core::time::Duration;
use core::{error::Error, mem};
use gio::prelude::FileExt;
use gtk::gio;
use rand::random_range;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::{fs, thread};
use tokio::sync::mpsc as tokio_mpsc;

pub mod album;
pub mod artist;
pub mod config;
pub mod search;
pub mod song;

pub use album::{Album, SharedAlbum, SortedAlbumSongs};
pub use artist::{Artist, SharedArtist, SortedArtistAlbums};
pub use config::{FILE_SUPPORT, LibraryConfig};
pub use song::{SharedSong, SharedSongExt, Song, SongInfo, SongInfoLoader};

use crate::excuses::{EXP_INIT, EXP_RX, INIT_ERR};
use crate::player::{PlayerRequest, QueueItem, SongQueue};
use crate::tasks::{BoxedTask, Runner};
use crate::ui::{UI_TX, UpdateUI};
use crate::{CONFIG_DIR, visit_dirs};

type LibraryTask = Box<dyn FnOnce(&Library) + Send + 'static>;

pub struct Library {
    pub songs: Songs,
    pub albums: Albums,
    pub artists: Artists,
    pub missing_songs: Songs,

    queue_initialized: bool,
    cancel_pending: Arc<AtomicBool>,

    on_albums_set: Vec<LibraryTask>,
    on_artists_set: Vec<LibraryTask>,

    tasks: Runner,
    pub config: LibraryConfig,
    pub player_tx: mpsc::Sender<PlayerRequest>,
    pub ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
    rx: mpsc::Receiver<LibraryRequest>,
}

pub trait ToQueue {
    fn to_queue(&self) -> Vec<QueueItem>;
}

pub trait ToShuffledQueue {
    fn to_shuffled_queue(&self) -> Vec<QueueItem>;
}

pub type Songs = Vec<Arc<Song>>;
pub trait SortedSongs {
    /// Returns `Ok(index)` if the item was found found
    ///
    /// # Errors
    /// If the item was not found, the returned `Err(index)`
    /// can be used to insert the item to the proper position
    fn find_song(&self, uri: &str, trim_start: usize) -> Result<usize, usize>;
}
impl SortedSongs for Songs {
    #[inline]
    fn find_song(&self, uri: &str, trim_start: usize) -> Result<usize, usize> {
        self.binary_search_by(|song| song.info().file_uri()[trim_start..].cmp(&uri[trim_start..]))
    }
}
impl ToQueue for Songs {
    fn to_queue(&self) -> Vec<QueueItem> {
        self.iter().map(QueueItem::from_song).collect()
    }
}

pub type Albums = Vec<Arc<Mutex<Album>>>;
pub trait SortedAlbums {
    /// Returns `Ok(index)` if the item was found found
    ///
    /// # Errors
    /// If the item was not found, the returned `Err(index)`
    /// can be used to insert the item to the proper position
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
    /// Returns `Ok(index)` if the item was found found
    ///
    /// # Errors
    /// If the item was not found, the returned `Err(index)`
    /// can be used to insert the item to the proper position
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
    CancelRebuild,

    QueueFromPaths(Box<[String]>),

    AddLibrary(Box<str>),
    EditLibrary(Box<(usize, String)>),
    RemoveLibrary(usize),

    SetSongs(Songs),
    SetAlbums(Albums),
    SetArtists(Artists),
    SetMissingSongs(Songs),

    RunTask(BoxedTask),
    OnAlbumsSet(LibraryTask),
    OnArtistsSet(LibraryTask),
    Shutdown,
}

impl Library {
    /// Returns a new `Library` instance and initializes `LIBRARY_TX`
    ///
    /// # Panics
    /// The function panics if `LIBRARY_TX` has already been set
    /// prior to calling this function
    #[inline]
    #[must_use]
    pub fn init(
        config: LibraryConfig,
        player_tx: mpsc::Sender<PlayerRequest>,
        ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
    ) -> Library {
        let (tx, rx) = mpsc::channel();
        LIBRARY_TX.set(tx).map_err(|_| INIT_ERR).unwrap();
        let _ = ui_tx.send(UpdateUI::SetLibraryDirs(config.directories.clone().into()));

        Library {
            songs: Vec::new(),
            albums: Vec::new(),
            artists: Vec::new(),
            missing_songs: Vec::new(),

            queue_initialized: false,
            cancel_pending: Arc::new(AtomicBool::new(false)),

            on_albums_set: Vec::new(),
            on_artists_set: Vec::new(),

            tasks: Runner::new(4),
            config,
            player_tx,
            ui_tx,
            rx,
        }
    }

    /// Main loop for handling library requests
    ///
    /// # Errors
    /// The function may error upon handling a request,
    /// in most cases due to a closed channel receiver
    ///
    /// # Panics
    /// The function may panic upon handling a request if
    /// a poisoned `Mutex` is passed
    #[inline]
    pub fn request_handler(mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.rx.recv()? {
                LibraryRequest::Rebuild => self.discover_files(),
                LibraryRequest::CancelRebuild => self.cancel_library_build(),

                LibraryRequest::SetSongs(songs) => self.set_songs(songs),
                LibraryRequest::SetAlbums(albums) => self.set_albums(albums),
                LibraryRequest::SetArtists(artists) => self.set_artists(artists),
                LibraryRequest::SetMissingSongs(songs) => self.set_missing_songs(songs),

                LibraryRequest::QueueFromPaths(paths) => self.play_from_paths(&paths)?,

                LibraryRequest::AddLibrary(dir) => self.config.add_library(dir.to_string()),
                LibraryRequest::EditLibrary(args) => self.config.edit_library(args.0, args.1),
                LibraryRequest::RemoveLibrary(index) => self.config.remove_library(index),

                LibraryRequest::RunTask(task) => self.tasks.run(task),
                LibraryRequest::OnAlbumsSet(f) => self.on_albums_set.push(f),
                LibraryRequest::OnArtistsSet(f) => self.on_artists_set.push(f),

                #[allow(clippy::unit_arg)]
                LibraryRequest::Shutdown => return Ok(self.shutdown()),
            }
        }
    }

    /// Locates song files within the configured directories, assigns
    /// them to `self.songs`, inserts any new files into `self.songs`,
    /// then runs `create_connections()` in a background process. If
    /// uninitialized, the data from disk is used first. Song entries
    /// already present in `self.songs` are preserved, and only new
    /// songs are added.
    ///
    /// # Panics
    /// The function panics if `create_connections()` fails
    pub fn discover_files(&mut self) {
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
                    songs.insert(index, SharedSong::from_file(file));
                }
            })
            .inspect_err(|e| eprintln!("Error reading '{library_path}': {e}"));
        }
        self.songs.clone_from(&songs);

        self.tasks.run({
            let config = self.config.clone();
            let cancel = Arc::clone(&self.cancel_pending);
            let missing_songs = self.missing_songs.clone();
            move || {
                Library::create_connections(songs, missing_songs, &config, &cancel).expect(EXP_RX);
            }
        });
    }

    /// Creates connections between library `songs`/`albums`/`artists`, and
    /// validates `songs` using `validate_songs()` and `merge_moved_entries()`
    ///
    /// # Errors
    /// The function errors if either the library or UI channel receiver is closed
    ///
    /// # Panics
    /// The function panics if a `song`'s info field is in a poisoned state
    pub fn create_connections(
        mut songs: Songs,
        mut missing: Songs,
        config: &LibraryConfig,
        cancel: &Arc<AtomicBool>,
    ) -> Result<(), Box<dyn Error>> {
        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let ui_tx = UI_TX.get().expect(EXP_INIT);

        let possibly_moved = Library::validate_songs(&mut songs, &mut missing, config);

        let background_task_spawner = thread::spawn({
            let cancel = Arc::clone(cancel);
            let songs = songs.clone();
            move || {
                // Wait before starting background tasks in case they aren't needed
                thread::sleep(Duration::from_millis(100));
                if cancel.load(atomic::Ordering::Relaxed) {
                    return;
                }

                // Spawning more tasks may improve parallel distribution
                let num_tasks = 64;
                let chunk_size = songs.len() / num_tasks;
                for i in 0..num_tasks {
                    let cancel = Arc::clone(&cancel);
                    let songs = songs[chunk_size * i..chunk_size * (i + 1)].to_vec();
                    Library::run_task(library_tx, move || {
                        for song in songs {
                            if cancel.load(atomic::Ordering::Relaxed) {
                                return;
                            }
                            drop(song.info().try_load_basic());
                        }
                    });
                }
                #[cfg(debug_assertions)]
                println!("Loading song info in the background ({num_tasks} tasks queued)");
            }
        });

        library_tx.send(LibraryRequest::SetMissingSongs(missing))?;
        Library::merge_moved_entries(&songs, possibly_moved, config, cancel);

        let mut albums = Vec::with_capacity(songs.len() / 16);
        let mut artists = Vec::with_capacity(songs.len() / 64);

        const PROGRESS_BAR_STEPS: usize = 320;
        let mut progress_interval = songs.len() / PROGRESS_BAR_STEPS + 1;
        progress_interval -= songs.len() % progress_interval;
        let progress_step = progress_interval as f64 / songs.len() as f64;
        let mut progress = 0.0;
        let mut iter = 0;

        for song in &songs {
            let mut info = song.info();
            let song_info = info.load_basic();
            // SAFETY: `load_basic` ensures the value is `Some`
            let song_info = unsafe { song_info.as_ref().unwrap_unchecked() };

            let album_index = albums.find_album(song_info);
            let artist_index = artists.find_artist(song_info);

            match artist_index {
                Ok(artist_index) => match album_index {
                    Ok(album_index) => {
                        // SAFETY: `album_index` is `Ok`, therefore within bounds
                        let album = unsafe { albums.get_unchecked(album_index) };
                        let album_songs = &mut album.lock().unwrap().songs;

                        // Add the song to the album songs
                        let song_index = album_songs.find_album_song(song_info);
                        match song_index {
                            Err(song_index) | Ok(song_index) => {
                                album_songs.insert(song_index, Arc::clone(song));
                            }
                        }

                        // Associate the song with its album
                        song.set_album(Arc::clone(album));
                    }
                    Err(album_index) => {
                        // SAFETY: `artist_index` is `Ok`, therefore within bounds
                        let artist = unsafe { artists.get_unchecked(artist_index) };
                        let artist_albums = &mut artist.lock().unwrap().albums;
                        let album = Arc::new(Mutex::new(Album {
                            title: song_info.album.clone(),
                            year: song_info.year,
                            songs: vec![Arc::clone(song)],
                            artist: Arc::clone(artist),
                        }));

                        // Add the album to `albums` and the artist's albums
                        albums.insert(album_index, Arc::clone(&album));
                        let album_index = artist_albums.find_artist_album(song_info);
                        match album_index {
                            Err(album_index) | Ok(album_index) => {
                                artist_albums.insert(album_index, Arc::clone(&album));
                            }
                        }

                        // Associate the song with its album
                        song.set_album(Arc::clone(&album));
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
                    song.set_album(album);
                }
            }

            iter += 1;
            if iter == progress_interval {
                if cancel.load(atomic::Ordering::Relaxed) {
                    let _ = ui_tx.send(UpdateUI::Progress(None));
                    break;
                }
                progress += progress_step;
                let _ = ui_tx.send(UpdateUI::Progress(Some(progress)));
                iter = 0;
            }
        }

        library_tx.send(LibraryRequest::SetArtists(artists))?;
        library_tx.send(LibraryRequest::SetAlbums(albums))?;
        library_tx.send(LibraryRequest::SetSongs(songs))?;

        ui_tx.send(UpdateUI::Progress(None))?;

        // Cancel background tasks if the main function finished first
        if !cancel.load(atomic::Ordering::Relaxed) {
            let _ = library_tx.send(LibraryRequest::CancelRebuild);
        }
        // Waiting may not be required, but it's okay since the function
        // is on a different thread, and has already updated the library
        let _ = background_task_spawner.join();

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
            let info = song.info();
            let missing_libraries = config.directories.iter().filter_map(|dir| {
                match fs::exists(dir).unwrap_or(false) {
                    false => Some(gio::File::for_path(dir).uri()),
                    true => None,
                }
            });
            match songs.find_song(&info.file_uri(), config.uri_opt()) {
                // Valid song entry
                Err(index)
                    if (info.file().path())
                        .is_some_and(|path| fs::exists(path).is_ok_and(|exists| exists)) =>
                {
                    for dir in &config.directories {
                        // Filter songs outside of `config.directories`
                        if !info.file_path().starts_with(dir) {
                            continue;
                        }
                        drop(info);
                        songs.insert(index, song);
                        continue 'iter;
                    }
                    // IDEA: To disable libraries, move `songs` into `disabled_songs`
                    drop(info);
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
                                    #[cfg(debug_assertions)]
                                    println!(
                                        "Remembering {} because its library is missing",
                                        info.filename()
                                    );
                                    drop(info);
                                    missing.insert(index, song);
                                    continue 'iter;
                                }
                            }
                            drop(info);
                            possibly_moved.push(song);
                        }
                        // Duplicate missing song entry
                        Ok(index) => {
                            info.user().merge_with(&missing[index].info().user());
                            drop(info);
                            drop(song);
                        }
                    }
                }
                // Duplicate entry
                Ok(index) => {
                    println!("Resolving duplicate entry: {}", info.filename());
                    info.user().merge_with(&songs[index].info().user());
                    drop(info);
                    drop(song);
                }
            }
        }

        // Check for file modifications in the background
        let check_songs = songs.clone();
        let library_tx = LIBRARY_TX.get().expect(EXP_RX);
        Library::run_task(library_tx, move || {
            let mut needs_rebuild = false;
            for song in check_songs {
                let mut info = song.info();
                let file_modification_time = info.file_modification_time();
                if file_modification_time == info.known_modification_time() {
                    continue;
                }
                #[cfg(debug_assertions)]
                if info.known_modification_time() != 0 {
                    // Only print if it isn't a new file
                    println!("{}: reloading info", info.filename());
                }
                let mut basic = info.inspect_basic_mut();
                if basic.is_some() {
                    *basic = None;
                    drop(basic);
                    needs_rebuild = true;
                    info.set_modification_time(file_modification_time);
                }
            }
            if needs_rebuild {
                // If files were modified, cancel and rebuild so the new info gets loaded
                let _ = library_tx.send(LibraryRequest::CancelRebuild);
                library_tx.send(LibraryRequest::Rebuild).expect(EXP_RX);
                println!("Modifications detected, restarting library build");
            }
        });

        possibly_moved
    }

    /// Attempts to locate missing files if they were moved and merges
    /// them with the existing song entries so their info is preserved
    ///
    /// # Panics
    /// The function may panic if the UI channel receiver is unititialized or closed
    pub fn merge_moved_entries(
        songs: &Songs,
        possibly_moved: Songs,
        config: &LibraryConfig,
        cancel: &Arc<AtomicBool>,
    ) {
        fn merge_if_matching(info: &mut SongInfoLoader, cmp_info: &SongInfoLoader) -> bool {
            if cmp_info.inspect_basic().eq(&info.load_basic()) {
                // Copy the user-assigned song info to the new entry
                println!("Found moved file: {}", cmp_info.filename());
                info.user().merge_with(&cmp_info.user());
                return true;
            }
            false
        }

        if possibly_moved.is_empty() {
            return;
        }

        let ui_tx = UI_TX.get().expect(EXP_INIT);

        const PROGRESS_BAR_STEPS: usize = 320;
        let mut progress_interval = possibly_moved.len() / PROGRESS_BAR_STEPS + 1;
        progress_interval -= possibly_moved.len() % progress_interval;
        let progress_step = progress_interval as f64 / possibly_moved.len() as f64;
        let mut progress = 0.0;
        let mut iter = 0;

        for missing in possibly_moved {
            let old_info = missing.info();

            // Optimization: start with an initial guess and expand outwards
            let guess = match songs.find_song(&old_info.file_uri(), config.uri_opt()) {
                Err(index) | Ok(index) => index,
            };
            let (mut left, mut right) = (songs[0..guess].iter(), songs[guess..].iter());
            loop {
                let (left, right) = (left.next_back(), right.next());
                if right.is_some_and(|song| merge_if_matching(&mut song.info(), &old_info))
                    || left.is_some_and(|song| merge_if_matching(&mut song.info(), &old_info))
                    || (left.is_none() && right.is_none())
                {
                    break;
                }
            }

            iter += 1;
            if iter == progress_interval {
                if cancel.load(atomic::Ordering::Relaxed) {
                    let _ = ui_tx.send(UpdateUI::Progress(None));
                    break;
                }
                progress += progress_step;
                let _ = ui_tx.send(UpdateUI::Progress(Some(progress)));
                iter = 0;
            }
        }
    }

    /// Cancels any currently running library build operation
    pub fn cancel_library_build(&self) {
        self.cancel_pending.store(true, atomic::Ordering::Relaxed);
        let _ = self.tasks.await_all_tasks();
        let cancel_pending = Arc::clone(&self.cancel_pending);
        self.tasks.run(move || {
            cancel_pending.store(false, atomic::Ordering::Relaxed);
        });
    }

    /// Uses `library_tx` to send the `task` to run on the thread pool.
    /// If idle threads are available, the `task` will run when the
    /// library processes the request, otherwise, it will wait in a queue.
    #[inline]
    pub fn run_task<T>(library_tx: &mpsc::Sender<LibraryRequest>, task: T)
    where
        T: FnOnce() + Into<Box<T>> + Send + 'static,
    {
        if let Err(e) = library_tx.send(LibraryRequest::RunTask(task.into())) {
            eprintln!("Could not run task: {e}");
        }
    }

    /// Replaces `self.songs` with `songs`
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    #[inline]
    fn set_songs(&mut self, songs: Songs) {
        self.ui_tx
            .send(UpdateUI::SetLibrarySongs(songs.clone()))
            .expect(EXP_RX);
        self.songs = songs;
    }
    /// Replaces `self.albums` with `albums`
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    #[inline]
    fn set_albums(&mut self, albums: Albums) {
        self.ui_tx
            .send(UpdateUI::SetLibraryAlbums(albums.clone()))
            .expect(EXP_RX);
        self.albums = albums;
        for f in mem::take(&mut self.on_albums_set) {
            f(self);
        }
    }
    /// Replaces `self.artists` with `artists`
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    #[inline]
    fn set_artists(&mut self, artists: Artists) {
        self.ui_tx
            .send(UpdateUI::SetLibraryArtists(artists.clone()))
            .expect(EXP_RX);
        self.artists = artists;
        for f in mem::take(&mut self.on_artists_set) {
            f(self);
        }
    }
    /// Replaces `self.missing_songs` with `missing_songs`
    #[inline]
    fn set_missing_songs(&mut self, missing_songs: Songs) {
        self.missing_songs = missing_songs;
    }

    /// Starts the initial player queue (see `SongQueue::init_queue` for more details)
    ///
    /// # Errors
    /// Function may error if the player or UI channel receiver is closed
    #[inline]
    pub fn init_queue(&self, queue_startup_choice: i32) -> Result<(), Box<dyn Error>> {
        match self.queue_initialized {
            false => SongQueue::init_queue(&self.config.dir, self, queue_startup_choice.into()),
            true => Ok(()),
        }
    }

    /// Starts a queue of all songs in the library
    ///
    /// # Errors
    /// The function errors if either the player or UI channel receiver is closed
    pub fn play_all_songs(&self, shuffle: bool) -> Result<(), Box<dyn Error>> {
        self.player_tx.send(PlayerRequest::LoadQueue(
            self.songs.to_queue(),
            match shuffle {
                true => Some(vec![]),
                false => None,
            },
            0,
        ))?;
        self.player_tx.send(PlayerRequest::TogglePlay(Some(true)))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Starts a queue of all albums in the library
    ///
    /// # Errors
    /// The function errors if either the player or UI channel receiver is closed
    pub fn play_all_albums(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.albums.to_queue(), None, 0))?;
        self.player_tx.send(PlayerRequest::TogglePlay(Some(true)))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Starts a randomly ordered queue of all albums in the library
    ///
    /// # Errors
    /// The function errors if either the player or UI channel receiver is closed
    pub fn shuffle_all_albums(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx.send(PlayerRequest::LoadQueue(
            self.albums.to_shuffled_queue(),
            None,
            0,
        ))?;
        self.player_tx.send(PlayerRequest::TogglePlay(Some(true)))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Starts a queue of all albums in the library
    ///
    /// # Errors
    /// The function errors if either the player or UI channel receiver is closed
    pub fn play_all_artists(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.artists.to_queue(), None, 0))?;
        self.player_tx.send(PlayerRequest::TogglePlay(Some(true)))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Starts a randomly ordered queue of all artists in the library
    ///
    /// # Errors
    /// The function errors if either the player or UI channel receiver is closed
    pub fn shuffle_all_artists(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx.send(PlayerRequest::LoadQueue(
            self.artists.to_shuffled_queue(),
            None,
            0,
        ))?;
        self.player_tx.send(PlayerRequest::TogglePlay(Some(true)))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        Ok(())
    }

    /// Starts a queue of all songs found within the specified `paths`,
    /// recursively. Does nothing if no song files were found.
    ///
    /// # Errors
    /// The function errors if either the player or UI channel receiver is closed
    pub fn play_from_paths(&mut self, paths: &[String]) -> Result<(), Box<dyn Error>> {
        let queue = self.songs_from_paths(paths);
        if queue.is_empty() {
            return Ok(());
        }
        self.player_tx
            .send(PlayerRequest::LoadQueue(queue, None, 0))?;
        self.player_tx.send(PlayerRequest::TogglePlay(Some(true)))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false))?;
        self.ui_tx.send(UpdateUI::FocusPlaying)?;
        self.queue_initialized = true;
        Ok(())
    }

    /// Takes a list of file or directory paths and returns a queue
    #[must_use]
    pub fn songs_from_paths(&self, paths: &[String]) -> Vec<QueueItem> {
        let mut queue = Vec::with_capacity(paths.len());
        for file in paths {
            if file_supported(file) {
                queue.push(QueueItem::Song(self.song_from_library_or_new(file)));
            } else if file == "Pause" {
                queue.push(QueueItem::new_stopper(false));
            } else if file == "Close Player" {
                queue.push(QueueItem::new_stopper(true));
            } else {
                self.extend_queue_from_dir(&mut queue, file);
            }
        }
        queue
    }
    /// Attempts to locate the given `file` within the library and
    /// returns it, otherwise it returns a new `SharedSong`
    #[inline]
    #[must_use]
    fn song_from_library_or_new(&self, file: &str) -> SharedSong {
        for dir in &self.config.directories {
            if file.starts_with(dir) {
                let file = gio::File::for_path(file);
                if let Ok(index) = self.songs.find_song(&file.uri(), self.config.uri_opt()) {
                    // SAFETY: `index` is `Ok`, therefore within bounds
                    return Arc::clone(unsafe { self.songs.get_unchecked(index) });
                }
                break;
            }
        }
        SharedSong::from_path(file)
    }
    /// Extends `queue` with songs found on disk within `dir`. If files are
    /// part of the music library, their existing instances will be used.
    ///
    /// The input `dir` must be a directory and exist on disk, otherwise
    /// the function does nothing.
    ///
    /// # Panics
    /// The function panics if any contained file paths are not valid UTF-8
    fn extend_queue_from_dir(&self, queue: &mut Vec<QueueItem>, dir: &str) {
        let path = Path::new(&dir);
        if !path.is_dir() || !path.exists() {
            return;
        }
        let mut songs = Vec::with_capacity(16);
        let _ = visit_dirs(path, &mut |file| {
            let file = file.path();
            let file = file.to_str().unwrap();
            if !file_supported(file) {
                return;
            }

            let song = QueueItem::Song(self.song_from_library_or_new(file));
            match songs.binary_search_by(|existing: &QueueItem| {
                (existing.as_song().info().file_path()).cmp(&song.as_song().info().file_path())
            }) {
                Err(index) | Ok(index) => songs.insert(index, song),
            }
        });
        queue.extend(songs);
    }

    /// Serializes `songs` and writes the data to disk,
    /// so the library can be loaded faster next time
    ///
    /// Writes to a file called `songs` in `self.config.dir`
    #[inline]
    fn serialize_songs(songs: &Songs) {
        let serialized = songs
            .iter()
            .filter_map(|song| song.try_serlialize().map(|s| s + "\n"))
            .collect::<String>();
        match fs::write(
            [CONFIG_DIR.get().expect(EXP_INIT), "songs"].concat(),
            serialized.trim(),
        ) {
            Ok(()) => println!("Library song info has been successfully written to disk"),
            Err(e) => eprintln!("Problems writing the library state to disk: \n{e}"),
        }
    }

    /// Reads the serialized song info from disk and returns them,
    /// so they can be assigned directly to `self.songs`
    ///
    /// Reads from a file called `songs` in `self.config.dir`
    #[must_use]
    fn deserialize_songs(&self) -> Songs {
        let Ok(data) = fs::read_to_string([&self.config.dir, "songs"].concat()) else {
            return Vec::with_capacity(512); // Estimate to reduce reallocations
        };
        (data.split("\n\n").filter_map(SharedSong::deserialize)).collect()
    }

    /// Consumes `self`, writes the configuration to disk and shuts down gracefully
    pub fn shutdown(mut self) {
        self.cancel_pending.store(true, atomic::Ordering::Relaxed);
        let mut songs = mem::take(&mut self.songs);
        for missing in mem::take(&mut self.missing_songs) {
            // Re-insert missing songs so their info is kept
            let Err(index) = songs.find_song(&missing.info().file_uri(), self.config.uri_opt())
            else {
                continue;
            };
            songs.insert(index, missing);
        }
        Library::serialize_songs(&songs);
        self.tasks.shutdown();
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
