use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::{gdk, glib};
use std::cell::RefCell;

use crate::ui::song_object::SongData;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::SongObject)]
pub struct SongObject {
    #[property(name = "song", get, set, type = String, member = song)]
    #[property(name = "artist", get, set, type = String, member = artist)]
    #[property(name = "artwork", get, set, type = Option<gdk::Paintable>, member = artwork)]
    pub data: RefCell<SongData>,
}

#[glib::object_subclass]
impl ObjectSubclass for SongObject {
    const NAME: &str = "MellowSongObject";
    type Type = super::SongObject;
}

#[glib::derived_properties]
impl ObjectImpl for SongObject {}
