use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

use crate::library::Albums;

mod imp;

glib::wrapper! {
    pub struct LibraryAlbumsPage(ObjectSubclass<imp::LibraryAlbumsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for LibraryAlbumsPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibraryAlbumsPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn load_albums(&self, albums: &Albums) {
        println!("load_albums()");
        self.imp().load_albums(albums);
    }
}
