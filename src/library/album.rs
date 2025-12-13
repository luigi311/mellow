use crate::library::Song;

use std::sync::{Arc, Mutex};

pub struct Album {
    pub title: String,
    pub songs: Vec<Arc<Mutex<Song>>>,
    pub artist: usize,
}
