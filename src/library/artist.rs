use std::sync::{Arc, Mutex};

use crate::library::{Album, SongInfo};

pub struct Artist {
    pub name: String,
    pub albums: Vec<Arc<Mutex<Album>>>,
}

pub type ArtistAlbums = Vec<Arc<Mutex<Album>>>;
pub trait SortedArtistAlbums {
    fn find_artist_album(&self, info: &SongInfo) -> Result<usize, usize>;
}
impl SortedArtistAlbums for ArtistAlbums {
    fn find_artist_album(&self, info: &SongInfo) -> Result<usize, usize> {
        self.binary_search_by(|album| album.lock().unwrap().title.cmp(&info.album))
    }
}
