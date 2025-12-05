// TODO: Implement a data structure which allows serializing data
// (such as ratings) for each song/album in the library
// TODO: Implement song/album/artist search/filtering

use core::error::Error;
use gtk::gio::{self, prelude::FileExt};
use gtk::glib;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use tokio::sync::mpsc as tokio_mpsc;

use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::UpdateUI;
use crate::visit_dirs;

pub mod album;
pub mod artist;
pub mod song;

pub use album::Album;
pub use artist::Artist;
pub use song::{Song, SongInfo};

// I don't know if this is the right approach, but I will try...
//
// IDEA: Initialization implementation:
//
// - Go through all the files, ignoring directories
// - Load metadata (title, album, artist, etc)
//    - Create new entry for each new album/artist
//    - Assign index of artist to album and vice-versa
//    - Assign index of album to song and vice-versa
//    - Assign file path to song
// - Assign fields and return Library
// - It would be best if each array could serialize to disk
//
// The fields could initially be initialized as BTreeMap and
// converted using `.into()`, if the performance is better.
//
// NOTE: If a song is added/removed, the indices might shift,
// so the relations need to be tracked somehow between rebuilds
// (bonus points if it detects renamed/moved files)
//
// TODO: Efficient search/filter by tag, rating, etc. Use SQL?

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
        if self.directories.iter().any(|existing| dir == *existing) {
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
    pub songs: Vec<Arc<Mutex<Song>>>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,

    config: LibraryConfig,
    player_tx: mpsc::SyncSender<PlayerRequest>,
    ui_tx: tokio_mpsc::Sender<UpdateUI>,
    rx: mpsc::Receiver<LibraryRequest>,
}

pub enum LibraryRequest {
    QueueAllSongs,
    Rebuild,
    AddLibrary(String),
    RemoveLibrary(usize),
}

impl Library {
    // TODO: Load library to avoid rebuilding each time
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

    pub async fn rebuild(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Rebuilding the music library");
        let songs = Arc::new(Mutex::new(Some(Vec::new())));
        self.config.directories.iter().for_each(|library_path| {
            let _ = visit_dirs(Path::new(&library_path), &|f| {
                let file = gio::File::for_path(f.path().to_str().unwrap());
                if !Library::file_supported(&file.parse_name()) {
                    return;
                }

                let song = Arc::new(Mutex::new(Song::new(file, None)));

                songs.lock().unwrap().as_mut().unwrap().push(song);
            })
            .inspect_err(|e| println!("Error reading '{library_path}': {e}"));
        });

        let songs = songs.lock().unwrap().take().unwrap();
        self.songs = songs;

        let albums = Vec::new();
        let artists = Vec::new();

        const PROGRESS_BAR_STEPS: usize = 270; // IDEA: Use window width?
        let progress_freq = self.songs.len() / PROGRESS_BAR_STEPS + 1;
        for (i, song) in self.songs.iter().enumerate() {
            // TODO: Assign song info, but skip memory-heavy fields (artwork, etc)
            // let mut song = song.lock().unwrap();
            // let mut info = song.info();
            // let song_info = info.basic();

            // // TODO: Assign song/album/artist index relations

            // // TODO: Initialize album/artist
            // let album = Album {
            //     // TODO
            // };
            // let artist = Artist {
            //     // TODO
            // };
            //
            // albums.push(album);
            // artists.push(artist);

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
                LibraryRequest::QueueAllSongs => self.queue_all_songs().await?,
                LibraryRequest::Rebuild => self.rebuild().await?,
                LibraryRequest::AddLibrary(dir) => self.config.add_library(dir),
                LibraryRequest::RemoveLibrary(index) => self.config.remove_library(index),
            }
        }
    }

    pub async fn queue_all_songs(&self) -> Result<(), Box<dyn Error>> {
        self.player_tx
            .send(PlayerRequest::LoadQueue(self.all_songs()))?;
        self.ui_tx.send(UpdateUI::OpenSheet(false)).await?;
        self.ui_tx.send(UpdateUI::FocusPlaying).await?;
        Ok(())
    }

    #[inline]
    #[must_use]
    pub fn file_supported(file: &str) -> bool {
        let Some(extension) = file.rsplit_once('.').map(|s| s.1.to_lowercase()) else {
            return false;
        };
        FILE_SUPPORT.iter().any(|&ext| extension == ext)
    }

    #[must_use]
    pub fn all_songs(&self) -> Vec<QueueItem> {
        self.songs
            .iter()
            .map(|song| QueueItem::Song(Arc::clone(song)))
            .collect()
    }

    #[must_use]
    pub fn songs_from_paths<P>(paths: &mut P) -> Option<Vec<QueueItem>>
    where
        P: Iterator<Item = String>,
    {
        let queue = Arc::new(Mutex::new(Some(Vec::new())));
        paths.for_each(|file| {
            let path = Path::new(&file);
            if path.is_file() {
                // Add files from arguments to queue
                if !Library::file_supported(&file) {
                    return;
                }
                let song = Song::new_from_str(&file, None);
                let song = QueueItem::Song(Arc::new(Mutex::new(song)));
                queue.lock().unwrap().as_mut().unwrap().push(song);
            } else if Path::exists(path) {
                // Add all files within directory arguments to queue
                let song_files = Arc::new(Mutex::new(Vec::new()));
                let _ = visit_dirs(path, &|file| {
                    let file = file.path();
                    let file = file.to_str().unwrap();
                    if !Library::file_supported(file) {
                        return;
                    }
                    song_files.lock().unwrap().push(file.to_owned());
                });
                let mut song_files = song_files.lock().unwrap();
                song_files.sort();
                song_files.iter().for_each(|file| {
                    let song = Song::new_from_str(file, None);
                    let song = QueueItem::Song(Arc::new(Mutex::new(song)));
                    queue.lock().unwrap().as_mut().unwrap().push(song);
                });
            }
        });

        match queue.lock().unwrap().take() {
            Some(queue) if !queue.is_empty() => Some(queue),
            _ => None,
        }
    }
}
