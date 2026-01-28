use glib::Object;
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct SongObject(ObjectSubclass<imp::SongObject>);
}

impl SongObject {
    pub fn new(song: &str, artist: &str) -> Self {
        Object::builder()
            .property("song", song)
            .property("artist", artist)
            .build()
    }
}

#[derive(Default)]
pub struct SongData {
    song: String,
    artist: String,
    artwork: Option<gdk::Paintable>,
}
