use std::sync::{Arc, Mutex};

use crate::library::{Artist, Song, SongInfo};

pub struct Album {
    pub title: String,
    pub year: u32,
    pub songs: AlbumSongs,
    pub artist: Arc<Mutex<Artist>>,
}

pub type AlbumSongs = Vec<Arc<Mutex<Song>>>;
pub trait SortedAlbumSongs {
    fn find_album_song(&self, info: &SongInfo) -> Result<usize, usize>;
}
impl SortedAlbumSongs for AlbumSongs {
    fn find_album_song(&self, info: &SongInfo) -> Result<usize, usize> {
        self.binary_search_by(|song| {
            let mut song = song.lock().unwrap();
            let mut new_info = song.info();
            let new_info = new_info.basic();
            format!("{}_{}", new_info.disc, new_info.track)
                .cmp(&format!("{}_{}", info.disc, info.track))
        })
    }
}
