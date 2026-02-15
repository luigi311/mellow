use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::{gdk, glib};

use std::cell::{OnceCell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{library::song::SharedSong, ui::song_object::SongData};

#[derive(Properties, Default)]
#[properties(wrapper_type = super::SongObject)]
pub struct SongObject {
    #[property(name = "index", get, set, type = u32, member = index)]
    #[property(name = "song", get, set, type = String, member = song)]
    #[property(name = "artist", get, set, type = String, member = artist)]
    #[property(name = "artwork", get, set, type = Option<gdk::Paintable>, member = artwork)]
    #[property(name = "rank", get, set, type = f64, member = rank)]
    pub data: RefCell<SongData>,

    pub shared_song: OnceCell<SharedSong>,
    pub is_visible: Arc<AtomicBool>,
}

#[glib::object_subclass]
impl ObjectSubclass for SongObject {
    const NAME: &str = "MellowSongObject";
    type Type = super::SongObject;
}

#[glib::derived_properties]
impl ObjectImpl for SongObject {}

impl Drop for SongObject {
    fn drop(&mut self) {
        self.is_visible.store(false, Ordering::Relaxed);
    }
}
