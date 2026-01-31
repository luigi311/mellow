use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct QueueSubpage(ObjectSubclass<imp::QueueSubpage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for QueueSubpage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl QueueSubpage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_info(&self, index: usize, song: &str, album: &str, artist: &str) {
        let song_page = self.imp();
        song_page.index.set(index);
        song_page.song_title.set_label(song);
        song_page.album_title.set_label(album);
        song_page.artist_name.set_label(artist);
        song_page.rating.connect_rating_set(|rating| {
            println!("TODO: Set rating to {rating}");
        });
    }

    pub fn set_stop_after(&self, stop_after: bool) {
        let song_page = self.imp();
        song_page.stop_after.set(stop_after);
        song_page.stop_after_button.set_title(match stop_after {
            // TODO: Support translations
            true => "Do Not Pause After",
            false => "Pause After",
        });
    }
}
