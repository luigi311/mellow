use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

use crate::library::artist::SharedArtist;
use crate::library::{Song, SongInfo, ToQueue};
use crate::player::queue_item::QueueItem;

// TODO: Save/load album info (such as ratings)

pub struct Album {
    pub title: String,
    pub year: u32,
    pub songs: AlbumSongs,
    pub artist: ArtistMutex,
}

impl ToQueue for Album {
    fn to_queue(&self) -> Vec<QueueItem> {
        self.songs.to_queue()
    }
}

pub type SharedAlbum = Arc<Mutex<Album>>;
impl ToQueue for SharedAlbum {
    fn to_queue(&self) -> Vec<QueueItem> {
        self.lock().unwrap().to_queue()
    }
}

pub type AlbumSongs = Vec<Arc<Mutex<Song>>>;
pub trait SortedAlbumSongs {
    /// Returns `Ok(index)` if found, or `Err(index)` if not
    fn find_album_song(&self, info: &SongInfo) -> Result<usize, usize>;
}
impl SortedAlbumSongs for AlbumSongs {
    #[inline]
    fn find_album_song(&self, info: &SongInfo) -> Result<usize, usize> {
        self.binary_search_by(|song| {
            let mut song = song.lock().unwrap();
            let mut new_info = song.info();
            let new_info = new_info.basic();

            match new_info.disc.cmp(&info.disc) {
                Ordering::Equal => new_info.track.cmp(&info.track),
                ordering => ordering,
            }
        })
    }
}
