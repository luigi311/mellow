use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

use crate::library::{Album, SongInfo, ToQueue};
use crate::player::queue_item::QueueItem;

pub struct Artist {
    pub name: String,
    pub albums: ArtistAlbums,
}

impl ToQueue for Artist {
    fn to_queue(&self) -> Vec<QueueItem> {
        let mut queue = Vec::<QueueItem>::with_capacity(16);
        for album in &self.albums {
            for song in &album.lock().unwrap().songs {
                queue.push(QueueItem::Song(Arc::clone(song)));
            }
        }
        queue
    }
}

pub type ArtistMutex = Arc<Mutex<Artist>>;
impl ToQueue for ArtistMutex {
    fn to_queue(&self) -> Vec<QueueItem> {
        self.lock().unwrap().to_queue()
    }
}

pub type ArtistAlbums = Vec<Arc<Mutex<Album>>>;
pub trait SortedArtistAlbums {
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
