use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::glib;
use std::cell::RefCell;

use crate::ui::album_object::AlbumData;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::AlbumObject)]
pub struct AlbumObject {
    #[property(name = "album", get, set, type = String, member = album)]
    #[property(name = "artist", get, set, type = String, member = artist)]
    pub data: RefCell<AlbumData>,
}

#[glib::object_subclass]
impl ObjectSubclass for AlbumObject {
    const NAME: &str = "MellowAlbumObject";
    type Type = super::AlbumObject;
}

#[glib::derived_properties]
impl ObjectImpl for AlbumObject {}
