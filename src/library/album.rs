use core::cmp::Ordering;
use std::sync::{Arc, Mutex};

use crate::library::{SharedArtist, Song, SongInfo, ToQueue};
use crate::player::QueueItem;

// TODO: Save/load album info (such as ratings)

pub struct Album {
    pub title: String,
    pub year: u16,
    pub songs: AlbumSongs,
    pub artist: SharedArtist,
}

impl Album {
    /// Loops through all album songs and returns the average rating,
    /// or returns `fallback` if no songs have a rating assigned. Songs
    /// with no rating assigned do not contribute to the average.
    #[must_use]
    pub fn average_rating(&self, fallback: f64) -> f64 {
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
            0 => fallback,
            n => rating_total / n as f64,
        }
    }
    /// Loops through all album songs and returns the average rating,
    /// defaulting to `fallback` for songs which do not have a rating
    /// assigned
    #[must_use]
    pub fn sort_rating(&self, fallback: f64) -> f64 {
        let mut rating_total = 0.0;
        for song in &self.songs {
            match song.info().user().rating {
                0 => rating_total += fallback,
                n => rating_total += n as f64,
            }
        }
        rating_total / self.songs.len() as f64
    }
    /// Loops through all album songs and returns the average play count
    #[must_use]
    pub fn average_play_count(&self) -> f64 {
        let mut play_count_total = 0;
        for song in &self.songs {
            play_count_total += song.info().user().play_count;
        }
        match self.songs.len() {
            0 => 0.0,
            n => play_count_total as f64 / n as f64,
        }
    }

    /// Sets the rating of all songs on the album to `rating`
    pub fn rate_all_songs(&self, rating: u8) {
        for song in &self.songs {
            song.info().set_rating(rating);
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
    /// Returns `Ok(index)` if the item was found found
    ///
    /// # Errors
    /// If the item was not found, the returned `Err(index)`
    /// can be used to insert the item to the proper position
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
