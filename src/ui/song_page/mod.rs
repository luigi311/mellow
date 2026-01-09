use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct SongPage(ObjectSubclass<imp::SongPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for SongPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl SongPage {
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
    }
}
