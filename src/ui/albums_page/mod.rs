use adw::subclass::prelude::*;
use gtk::{gdk, glib};

use crate::library::Albums;

mod imp;

glib::wrapper! {
    pub struct AlbumsPage(ObjectSubclass<imp::AlbumsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl AlbumsPage {
    pub fn init_search(&self) {
        self.imp().init_search();
    }

    pub fn load_albums(&self, albums: &Albums) {
        self.imp().load_albums(albums);
    }

    #[inline]
    pub fn assign_artwork(&self, index: u32, artwork: Option<&gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    pub fn uninit(&self) {
        self.imp().uninit();
    }
}
