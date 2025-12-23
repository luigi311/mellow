use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

use crate::library::Songs;

mod imp;

glib::wrapper! {
    pub struct LibrarySongsPage(ObjectSubclass<imp::LibrarySongsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for LibrarySongsPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibrarySongsPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn load_songs(&self, songs: &Songs) {
        println!("load_songs()");
        self.imp().load_songs(songs);
    }
}
