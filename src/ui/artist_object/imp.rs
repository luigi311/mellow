use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::{gdk, glib};
use std::cell::{OnceCell, RefCell};

use crate::library::SharedArtist;
use crate::ui::artist_object::ArtistData;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::ArtistObject)]
pub struct ArtistObject {
    #[property(name = "index", get, set, type = u32, member = index)]
    #[property(name = "artist", get, set, type = String, member = artist)]
    #[property(name = "albums", get, set, type = u64, member = albums)]
    #[property(name = "artwork", get, set, type = Option<gdk::Paintable>, member = artwork)]
    #[property(name = "rank", get, set, type = f64, member = rank)]
    pub data: RefCell<ArtistData>,

    pub shared_artist: OnceCell<SharedArtist>,
}

#[glib::object_subclass]
impl ObjectSubclass for ArtistObject {
    const NAME: &str = "MellowArtistObject";
    type Type = super::ArtistObject;
}

#[glib::derived_properties]
impl ObjectImpl for ArtistObject {}
