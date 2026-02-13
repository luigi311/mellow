use adw::subclass::prelude::*;
use gtk::{gdk, glib};

use crate::library::Artists;

mod imp;

glib::wrapper! {
    pub struct ArtistsPage(ObjectSubclass<imp::ArtistsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl ArtistsPage {
    pub fn init_search(&self) {
        self.imp().init_search();
    }

    pub fn load_artists(&self, artists: &Artists) {
        println!("load_artists()");
        self.imp().load_artists(artists);
    }

    pub fn assign_artwork(&self, index: u32, artwork: Option<gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    pub fn uninit(&self) {
        self.imp().uninit();
    }
}
