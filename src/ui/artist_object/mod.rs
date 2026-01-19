use glib::Object;
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct ArtistObject(ObjectSubclass<imp::ArtistObject>);
}

impl ArtistObject {
    pub fn new(artist: &str, albums: u64) -> Self {
        Object::builder()
            .property("artist", artist)
            .property("albums", albums)
            .build()
    }
}

#[derive(Default)]
pub struct ArtistData {
    artist: String,
    albums: u64,
    artwork: Option<gdk::Paintable>,
}
