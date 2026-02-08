use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::{gdk, glib};
use std::cell::{OnceCell, RefCell};

use crate::{library::song::SharedSong, ui::song_object::SongData};

#[derive(Properties, Default)]
#[properties(wrapper_type = super::SongObject)]
pub struct SongObject {
    #[property(name = "index", get, set, type = u32, member = index)]
    #[property(name = "song", get, set, type = String, member = song)]
    #[property(name = "artist", get, set, type = String, member = artist)]
    #[property(name = "artwork", get, set, type = Option<gdk::Paintable>, member = artwork)]
    pub data: RefCell<SongData>,

    pub first_song: OnceCell<SharedSong>,
}

#[glib::object_subclass]
impl ObjectSubclass for SongObject {
    const NAME: &str = "MellowSongObject";
    type Type = super::SongObject;
}

#[glib::derived_properties]
impl ObjectImpl for SongObject {}
