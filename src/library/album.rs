use std::sync::{Arc, Mutex};

use crate::library::{Artist, Song};

pub struct Album {
    pub title: String,
    pub year: String,
    pub songs: Vec<Arc<Mutex<Song>>>,
    pub artist: Arc<Mutex<Artist>>,
}
