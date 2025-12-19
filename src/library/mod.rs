use core::error::Error;
use gio::prelude::FileExt;
use gtk::{gio, glib};
use rand::random_range;
use std::mem;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use tokio::sync::mpsc as tokio_mpsc;

pub mod album;
pub mod artist;
pub mod song;

pub use album::Album;
pub use artist::Artist;
pub use song::{Song, SongInfo};

use crate::excuses::EXP_INIT;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::UpdateUI;
use crate::visit_dirs;

// TODO: Implement a data structure which allows serializing data
// (such as ratings) for each song/album in the library
// TODO: Implement song/album/artist search/filtering
// TODO: Efficient search/filter by tag, rating, titles, etc. Use SQL?

const FILE_SUPPORT: &[&str] = &[
    "flac", "m4a", "mp3", "aac", "ac3", "wav",
    // TODO: Ensure all listed formats work
    // Untested:
    "ape", "mpc", "ogg",
];

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

    pub fn add_library(&mut self, dir: String) {
        if self.directories.contains(&dir) {
            return;
        }
        self.directories.push(dir);
        println!("Added a new library\nLibraries: {:?}", self.directories);
    }

    pub fn remove_library(&mut self, index: usize) {
        self.directories.remove(index);
        println!("Removed a library\nLibraries: {:?}", self.directories);
    }
}

pub struct Library {
    /// All songs in the library, sorted by the relative part of the URI
    /// (`file:///path/to/library/Folder/File.mp3` => `Folder/File.mp3`)
    pub songs: Vec<Arc<Mutex<Song>>>,
    /// All albums in the library, sorted by title
    pub albums: Vec<Arc<Mutex<Album>>>,
    /// All artists in the library, sorted by name
    pub artists: Vec<Arc<Mutex<Artist>>>,

    config: LibraryConfig,
    player_tx: mpsc::SyncSender<PlayerRequest>,
    ui_tx: tokio_mpsc::Sender<UpdateUI>,
    rx: mpsc::Receiver<LibraryRequest>,
}

pub enum LibraryRequest {
    PlayAllSongs,
    PlayAllAlbums,
    ShuffleAllAlbums,
    PlayAllArtists,
    ShuffleAllArtists,
    Rebuild,
    AddLibrary(String),
    RemoveLibrary(usize),
}

impl Library {
    #[must_use]
    pub fn init(
        player_tx: mpsc::SyncSender<PlayerRequest>,
        ui_tx: tokio_mpsc::Sender<UpdateUI>,
    ) -> (Library, mpsc::SyncSender<LibraryRequest>) {
        let (tx, rx) = mpsc::sync_channel(4);
        (
            Library {
                songs: vec![],
                albums: vec![],
                artists: vec![],

                config: LibraryConfig::load(),
                player_tx,
                ui_tx,
                rx,
            },
            tx,
        )
    }

    /// Creates connections between library `songs`, `albums`, and `artists`
    #[allow(clippy::await_holding_lock)] // False-positive warning
    pub async fn rebuild(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Rebuilding the music library");

        // TODO: Initialize `songs` using serialized `SongInfo`
        // TODO: Check file modification times and update info/associations

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
                let index = songs.binary_search_by(|song| {
                    // Shortening the URI makes the lookup faster, however
                    // files with identical relative paths will be ignored
                    song.lock().unwrap().info().file_uri()[to_relative..]
                        .cmp(&file.uri()[to_relative..])
                });
                let Err(index) = index else {
                    return;
                };

                let song = Arc::new(Mutex::new(Song::new(file, None)));
                songs.insert(index, song);
            })
            .inspect_err(|e| println!("Error reading '{library_path}': {e}"));
        });
        let songs = songs.lock().unwrap().take().expect(EXP_INIT);

        self.songs = songs;

        // return Ok(());
        // TODO: Do the rest in a background thread, if possible

        let mut albums: Vec<Arc<Mutex<Album>>> = Vec::new();
        let mut artists: Vec<Arc<Mutex<Artist>>> = Vec::new();

        const PROGRESS_BAR_STEPS: usize = 270; // IDEA: Use window width?
        let progress_freq = self.songs.len() / PROGRESS_BAR_STEPS + 1;
        for (i, song) in self.songs.iter().enumerate() {
            let mut song_unwrapped = song.lock().unwrap();
            let mut info = song_unwrapped.info();
            let song_info = info.basic();

            // TODO: Improve `albums` sorting: artist/year/title or artist/title
            // TODO: Improve `artists[…].albums` sorting: year/title
            let album_index =
                albums.binary_search_by(|album| album.lock().unwrap().title.cmp(&song_info.album));

            let artist_index = artists.binary_search_by(|artist| {
                artist.lock().unwrap().name.cmp(&song_info.album_artist)
            });

            match artist_index {
                Ok(artist_index) => match album_index {
                    Ok(album_index) => {
                        // Associate the current song with its album
                        let album_songs = &mut albums[album_index].lock().unwrap().songs;
                        let song_index = album_songs.binary_search_by(|song| {
                            let mut song = song.lock().unwrap();
                            let mut new_info = song.info();
                            let new_info = new_info.basic();
                            format!("{}_{}", new_info.disc, new_info.track)
                                .cmp(&format!("{}_{}", song_info.disc, song_info.track))
                        });
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
                        let album_index = artist_albums.binary_search_by(|album| {
                            album.lock().unwrap().title.cmp(&song_info.title)
                        });
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
                        year: song_info.year.clone(),
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

        self.albums = albums;
        self.artists = artists;

        self.ui_tx.send(UpdateUI::Progress(None)).await?;
        Ok(())
    }

    pub async fn request_handler(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.rx.recv()? {
                LibraryRequest::PlayAllSongs => self.play_all_songs().await?,
                LibraryRequest::PlayAllAlbums => self.play_all_albums().await?,
                LibraryRequest::ShuffleAllAlbums => self.shuffle_all_albums().await?,
                LibraryRequest::PlayAllArtists => self.play_all_artists().await?,
                LibraryRequest::ShuffleAllArtists => self.shuffle_all_artists().await?,
                LibraryRequest::Rebuild => self.rebuild().await?,
                LibraryRequest::AddLibrary(dir) => self.config.add_library(dir),
                LibraryRequest::RemoveLibrary(index) => self.config.remove_library(index),
            }
        }
    }

    pub async fn play_all_songs(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_songs()))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
        Ok(())
    }

    pub async fn play_all_albums(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_albums()))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
        Ok(())
    }

    pub async fn shuffle_all_albums(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_albums_shuffled()))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
        Ok(())
    }

    pub async fn play_all_artists(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_artists()))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
        Ok(())
    }

    pub async fn shuffle_all_artists(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_artists_shuffled()))?;
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

    /// Returns a queue of all songs found within the specified `paths`,
    /// recursively. Returns `None` if no song files were found.
    #[must_use]
    pub fn songs_from_paths<P>(paths: &mut P) -> Option<Vec<QueueItem>>
    where
        P: Iterator<Item = String>,
    {
        let queue = Arc::new(Mutex::new(Some(Vec::new())));
        // IDEA: Queue the song from the library directly if
        // the file is within one of the configured directories
        paths.for_each(|file| {
            let path = Path::new(&file);
            if path.is_file() {
                // Add files from arguments to queue
                if !Library::file_supported(&file) {
                    return;
                }
                let song = Song::new_from_str(&file, None);
                let song = QueueItem::Song(Arc::new(Mutex::new(song)));
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

                    let mut songs = songs.lock().unwrap();
                    let songs = songs.as_mut().expect(EXP_INIT);

                    let song = Song::new_from_str(file, None);
                    let song = QueueItem::Song(Arc::new(Mutex::new(song)));

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
        });

        match queue.lock().unwrap().take() {
            Some(queue) if !queue.is_empty() => Some(queue),
            _ => None,
        }
    }
}
