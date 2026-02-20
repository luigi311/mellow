use adw::subclass::prelude::*;
use gtk::{gdk, glib};

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
        self.imp().load_songs(songs);
    }

    #[inline]
    pub fn assign_artwork(&self, index: u32, artwork: Option<&gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    pub fn uninit(&self) {
        self.imp().uninit();
    }
}
