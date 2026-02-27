use core::cmp::Ordering;
use std::sync::{Arc, Mutex};

use crate::library::{Album, SongInfo, ToQueue, ToShuffledQueue};
use crate::player::QueueItem;

pub struct Artist {
    pub name: String,
    pub albums: ArtistAlbums,
}

impl ToQueue for Artist {
    fn to_queue(&self) -> Vec<QueueItem> {
        self.albums.to_queue()
    }
}
impl ToShuffledQueue for Artist {
    fn to_shuffled_queue(&self) -> Vec<QueueItem> {
        self.albums.to_shuffled_queue()
    }
}

pub type SharedArtist = Arc<Mutex<Artist>>;
impl ToQueue for SharedArtist {
    fn to_queue(&self) -> Vec<QueueItem> {
        self.lock().unwrap().to_queue()
    }
}
impl ToShuffledQueue for SharedArtist {
    fn to_shuffled_queue(&self) -> Vec<QueueItem> {
        self.lock().unwrap().to_shuffled_queue()
    }
}

pub type ArtistAlbums = Vec<Arc<Mutex<Album>>>;
pub trait SortedArtistAlbums {
    /// Returns `Ok(index)` if found, or `Err(index)` if not
    fn find_artist_album(&self, info: &SongInfo) -> Result<usize, usize>;
}
impl SortedArtistAlbums for ArtistAlbums {
    fn find_artist_album(&self, info: &SongInfo) -> Result<usize, usize> {
        self.binary_search_by(|album| {
            let album = album.lock().unwrap();
            match album.year.cmp(&info.year) {
                Ordering::Equal => album.title.cmp(&info.album),
                ordering => ordering,
            }
        })
    }
}
