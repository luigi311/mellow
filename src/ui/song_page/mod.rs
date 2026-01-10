use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

use crate::library::{ToQueue, song::SongMutex};

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

    pub fn update(&self, index: usize, song: &SongMutex, to_queue: Box<dyn ToQueue + Send>) {
        let song_page = self.imp();

        song_page.index.set(index); // TODO!!
        let mut song = song.lock().unwrap();
        let mut info = song.info();
        let info = info.basic();
        song_page.song_title.set_label(&info.title);
        song_page.album_title.set_label(&info.album);
        song_page.artist_name.set_label(&info.artist);
        song_page.context.replace(Some(to_queue));
    }
}
