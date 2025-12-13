use std::sync::{Arc, Mutex};

use crate::library::Album;

pub struct Artist {
    pub name: String,
    pub albums: Vec<Arc<Mutex<Album>>>,
}
