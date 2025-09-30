// TODO: Implement a data structure which allows serializing data
// (such as ratings) for each song/album in the library
// TODO: Implement song/album/artist search/filtering

use core::error::Error;
use gtk::gio::{self, prelude::FileExt};
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::library::{Album, Artist, Song};

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
    ".flac", ".m4a", ".mp3", ".mpc", ".ogg", ".aac", ".aiff", ".ape", ".wav",
];

pub struct Library {
    pub songs: Vec<Song>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
}

impl Library {
    // TODO: Serialize to / deserialize from disk
    pub fn load_or_init() {}

    // TODO: Load in chunks, modify self, allow showing progress
    pub fn rebuild() -> Result<Library, Box<dyn Error>> {
        // TODO: Don't hardcode music library path
        let library_path = [std::env::home_dir().unwrap().to_str().unwrap(), "/Music/"].concat();

        let songs = Arc::new(Mutex::new(Some(Vec::new())));
        let albums = Arc::new(Mutex::new(Some(Vec::new())));
        let artists = Arc::new(Mutex::new(Some(Vec::new())));
        visit_dirs(&Path::new(&library_path), &|f| {
            let file = gio::File::for_path(f.path().to_str().unwrap().to_owned());

            let file_lcase = file.parse_name().to_lowercase();
            if !FILE_SUPPORT.iter().any(|ext| file_lcase.ends_with(ext)) {
                return;
            }

            let song = Song {
                file,
                album: None,
                info: None,
            };
            // TODO: Read song info - note that this will take a while,
            // so it's best to implement disk serialization first
            // song.get_info_or_assign();

            // TODO: Assign song/album/artist index relations

            // TODO: Initialize album/artist
            // let album = Album {
            //     // TODO
            // };
            // let artist = Artist {
            //     // TODO
            // };
            songs.lock().unwrap().as_mut().unwrap().push(song);
        })?;

        let songs = songs.lock().unwrap().take().unwrap();
        let albums = albums.lock().unwrap().take().unwrap();
        let artists = artists.lock().unwrap().take().unwrap();
        Ok(Library {
            songs,
            albums,
            artists,
        })
    }
    pub fn song_by_index(&self, index: usize) -> &Song {
        &self.songs[index]
    }
    pub fn album_by_index(&self, index: usize) -> &Album {
        &self.albums[index]
    }
    pub fn artist_by_index(&self, index: usize) -> &Artist {
        &self.artists[index]
    }
}

// one possible implementation of walking a directory only visiting files
fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}
