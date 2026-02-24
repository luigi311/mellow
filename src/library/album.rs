use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

use crate::library::artist::SharedArtist;
use crate::library::{Song, SongInfo, ToQueue};
use crate::player::queue_item::QueueItem;

// TODO: Save/load album info (such as ratings)

pub struct Album {
    pub title: String,
    pub year: u16,
    pub songs: AlbumSongs,
    pub artist: SharedArtist,
}

impl Album {
    pub fn compute_average_rating(&self, default_to: f64) -> f64 {
        let mut rating_total = 0.0;
        let mut num_songs = 0;
        for song in &self.songs {
            match song.info().user().rating {
                0 => continue,
                n => rating_total += n as f64,
            }
            num_songs += 1;
        }
        match num_songs {
            0 => default_to,
            n => rating_total / n as f64,
        }
    }
    pub fn compute_average_play_count(&self) -> f64 {
        let mut play_count_total = 0;
        for song in &self.songs {
            play_count_total += song.info().user().play_count;
        }
        match self.songs.len() {
            0 => 0.0,
            n => play_count_total as f64 / n as f64,
        }
    }
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

pub type AlbumSongs = Vec<Arc<Song>>;
pub trait SortedAlbumSongs {
    /// Returns `Ok(index)` if found, or `Err(index)` if not
    fn find_album_song(&self, info: &SongInfo) -> Result<usize, usize>;
}
impl SortedAlbumSongs for AlbumSongs {
    #[inline]
    fn find_album_song(&self, info: &SongInfo) -> Result<usize, usize> {
        self.binary_search_by(|song| {
            let mut new_info = song.info();
            let new_info = new_info.load_basic();
            // SAFETY: `load_basic` ensures the value is `Some`
            let new_info = unsafe { new_info.as_ref().unwrap_unchecked() };

            match new_info.disc.cmp(&info.disc) {
                Ordering::Equal => new_info.track.cmp(&info.track),
                ordering => ordering,
            }
        })
    }
}
