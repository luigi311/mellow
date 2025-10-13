// TODO: Implement a data structure which allows serializing data
// (such as ratings) for each song/album in the library
// TODO: Implement song/album/artist search/filtering

use core::error::Error;
use gtk::gio::{self, prelude::FileExt};
use gtk::glib;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc as tokio_mpsc;

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
    pub directories: Box<[String]>,
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

pub struct Library {
    pub songs: Vec<Song>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,

    config: LibraryConfig,
    ui_tx: tokio_mpsc::Sender<UpdateUI>,
}

impl Library {
    // TODO: Load library to avoid rebuilding each time
    pub fn load_or_init(ui_tx: tokio_mpsc::Sender<UpdateUI>) -> Result<Library, Box<dyn Error>> {
        // TODO: Load config from disk
        let config = LibraryConfig::default();

        Ok(Library {
            songs: vec![],
            albums: vec![],
            artists: vec![],

            config,
            ui_tx,
        })
    }

    pub async fn rebuild(&mut self) -> Result<(), Box<dyn Error>> {
        let songs = Arc::new(Mutex::new(Some(Vec::new())));
        self.config.directories.iter().for_each(|library_path| {
            let _ = visit_dirs(Path::new(&library_path), &|f| {
                let file = gio::File::for_path(f.path().to_str().unwrap());
                if !Library::file_supported(&file.parse_name()) {
                    return;
                }

                let song = Song {
                    file,
                    album: None,
                    info: None,
                };

                songs.lock().unwrap().as_mut().unwrap().push(song);
            })
            .inspect_err(|e| println!("Error reading '{library_path}': {e}"));
        });

        let albums = Arc::new(Mutex::new(Some(Vec::new())));
        let artists = Arc::new(Mutex::new(Some(Vec::new())));

        const PROGRESS_BAR_STEPS: usize = 270; // IDEA: Use window width?
        let songs = songs.lock().unwrap().take().unwrap();
        let progress_freq = songs.len() / PROGRESS_BAR_STEPS + 1;
        for i in 0..songs.len() {
            // TODO: Assign song info, but skip memory-heavy fields (artwork, etc)
            // songs[i].get_info_or_assign();

            // // TODO: Assign song/album/artist index relations

            // // TODO: Initialize album/artist
            // let album = Album {
            //     // TODO
            // };
            // let artist = Artist {
            //     // TODO
            // };

            if i % progress_freq == 0 {
                println!("{i}");
                self.ui_tx
                    .send(UpdateUI::Progress(Some(i as f64 / songs.len() as f64)))
                    .await?
            }
        }

        self.songs = songs;
        self.albums = albums.lock().unwrap().take().unwrap();
        self.artists = artists.lock().unwrap().take().unwrap();

        self.ui_tx.send(UpdateUI::Progress(None)).await?;
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
    pub fn song_by_index(&self, index: usize) -> &Song {
        &self.songs[index]
    }
    #[must_use]
    pub fn album_by_index(&self, index: usize) -> &Album {
        &self.albums[index]
    }
    #[must_use]
    pub fn artist_by_index(&self, index: usize) -> &Artist {
        &self.artists[index]
    }
}
