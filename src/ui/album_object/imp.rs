use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::{gdk, glib};
use std::cell::{OnceCell, RefCell};

use crate::{library::song::SharedSong, ui::album_object::AlbumData};

#[derive(Properties, Default)]
#[properties(wrapper_type = super::AlbumObject)]
pub struct AlbumObject {
    #[property(name = "index", get, set, type = u32, member = index)]
    #[property(name = "album", get, set, type = String, member = album)]
    #[property(name = "artist", get, set, type = String, member = artist)]
    #[property(name = "artwork", get, set, type = Option<gdk::Paintable>, member = artwork)]
    pub data: RefCell<AlbumData>,

    pub first_song: OnceCell<SharedSong>,
}

#[glib::object_subclass]
impl ObjectSubclass for AlbumObject {
    const NAME: &str = "MellowAlbumObject";
    type Type = super::AlbumObject;
}

#[glib::derived_properties]
impl ObjectImpl for AlbumObject {}
