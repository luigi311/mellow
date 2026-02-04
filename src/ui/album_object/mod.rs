use glib::Object;
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct AlbumObject(ObjectSubclass<imp::AlbumObject>);
}

impl AlbumObject {
    pub fn new(album: &str, artist: &str, artwork: Option<gdk::Texture>) -> Self {
        Object::builder()
            .property("album", album)
            .property("artist", artist)
            .property("artwork", artwork)
            .build()
    }
}

#[derive(Default)]
pub struct AlbumData {
    album: String,
    artist: String,
    artwork: Option<gdk::Texture>,
}
