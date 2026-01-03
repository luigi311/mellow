use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct QueueSongPage(ObjectSubclass<imp::QueueSongPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for QueueSongPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl QueueSongPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_info(&self, index: usize, song: &str, album: &str, artist: &str, stop_after: bool) {
        let song_page = self.imp();
        song_page.index.set(index);
        song_page.song_title.set_label(song);
        song_page.album_title.set_label(album);
        song_page.artist_name.set_label(artist);
        song_page.stop_after.set(stop_after);
        song_page.stop_after_button.set_title(match stop_after {
            // TODO: Support translations
            true => "Do Not Stop After",
            false => "Stop After",
        });
    }
}
