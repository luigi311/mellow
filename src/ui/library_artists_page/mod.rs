use adw::subclass::prelude::*;
use gtk::glib;

use crate::library::Artists;

mod imp;

glib::wrapper! {
    pub struct LibraryArtistsPage(ObjectSubclass<imp::LibraryArtistsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl LibraryArtistsPage {
    pub fn init_search(&self) {
        self.imp().init_search();
    }

    pub fn load_artists(&self, artists: &Artists) {
        println!("load_artists()");
        self.imp().load_artists(artists);
    }
}
