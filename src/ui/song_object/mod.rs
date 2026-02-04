use glib::Object;
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct SongObject(ObjectSubclass<imp::SongObject>);
}

impl SongObject {
    pub fn new(song: &str, artist: &str, artwork: Option<gdk::Texture>) -> Self {
        Object::builder()
            .property("song", song)
            .property("artist", artist)
            .property("artwork", artwork)
            .build()
    }
}

#[derive(Default)]
pub struct SongData {
    song: String,
    artist: String,
    artwork: Option<gdk::Texture>,
}
