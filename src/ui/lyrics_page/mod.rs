use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct LyricsPage(ObjectSubclass<imp::LyricsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for LyricsPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LyricsPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_content(&self, song_title: &str, lyrics: &str) {
        let lyrics_page = self.imp();
        lyrics_page.song_title.set_label(song_title);
        if lyrics.is_empty() {
            lyrics_page.lyrics.set_label("Lyrics not available");
        } else {
            lyrics_page.lyrics.set_label(lyrics);
        }
    }
}
