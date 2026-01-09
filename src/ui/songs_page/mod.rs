use adw::subclass::prelude::*;
use gtk::glib;

use crate::library::Songs;

mod imp;

glib::wrapper! {
    pub struct SongsPage(ObjectSubclass<imp::SongsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl SongsPage {
    pub fn init_search(&self) {
        self.imp().init_search();
    }

    pub fn load_songs(&self, songs: &Songs) {
        println!("load_songs()");
        self.imp().load_songs(songs);
    }
}
