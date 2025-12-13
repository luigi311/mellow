// TODO: Implement a data structure which allows serializing data
// (such as ratings) for each song/album in the library
// TODO: Implement song/album/artist search/filtering

use core::error::Error;
use gtk::gio::{self, prelude::FileExt};
use gtk::glib;
use rand::random_range;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use tokio::sync::mpsc as tokio_mpsc;

use crate::excuses::EXP_INIT;
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
    pub songs: Vec<Arc<Mutex<Song>>>,
    pub albums: Vec<Arc<Mutex<Album>>>,
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

                songs.lock().unwrap().as_mut().expect(EXP_INIT).push(song);
            })
            .inspect_err(|e| println!("Error reading '{library_path}': {e}"));
        });
        let songs = songs.lock().unwrap().take().expect(EXP_INIT);

        self.songs = songs;

        // TODO: Load the library to avoid rebuilding each time
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

            let artist_index = artists
                .binary_search_by(|artist| artist.lock().unwrap().name.cmp(&song_info.artist));
            let album_index =
                albums.binary_search_by(|album| album.lock().unwrap().title.cmp(&song_info.album));

            match artist_index {
                Ok(artist_index) => match album_index {
                    Ok(album_index) => {
                        let album_songs = &mut albums[album_index].lock().unwrap().songs;
                        let mut song_index = album_songs.len() - 1;
                        while song_index > 0 {
                            if album_songs[song_index].lock().unwrap().info().basic().track
                                < song_info.track
                            {
                                song_index -= 1
                            } else {
                                break;
                            }
                        }
                        album_songs.insert(song_index, Arc::clone(&song));
                    }
                    Err(album_index) => {
                        albums.insert(
                            album_index,
                            Arc::new(Mutex::new(Album {
                                title: song_info.album.clone(),
                                songs: vec![Arc::clone(&song)],
                                artist: 0, // TODO
                            })),
                        );
                    }
                },
                Err(artist_index) => {
                    artists.insert(
                        artist_index,
                        Arc::new(Mutex::new(Artist {
                            name: song_info.artist.clone(),
                            albums: [].into(), // TODO
                        })),
                    );
                }
            }

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
    pub fn all_albums(&self) -> Vec<QueueItem> {
        let mut queue = Vec::<QueueItem>::new();
        for album in &self.albums {
            for song in &album.lock().unwrap().songs {
                queue.push(QueueItem::Song(Arc::clone(song)));
            }
        }
        queue
    }

    #[must_use]
    pub fn all_albums_shuffled(&self) -> Vec<QueueItem> {
        // TODO: Create a queue of albums in a random order
        let mut queue = Vec::new();
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
                queue.lock().unwrap().as_mut().expect(EXP_INIT).push(song);
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
                    queue.lock().unwrap().as_mut().expect(EXP_INIT).push(song);
                });
            }
        });

        match queue.lock().unwrap().take() {
            Some(queue) if !queue.is_empty() => Some(queue),
            _ => None,
        }
    }
}
