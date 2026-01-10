use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;

use crate::library::Albums;

mod imp;

glib::wrapper! {
    pub struct ArtistPage(ObjectSubclass<imp::ArtistPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for ArtistPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ArtistPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn update(&self, index: usize, albums: &Albums) {
        let artist_page = self.imp();
        artist_page.index.set(index);

        println!("TODO: Show artist's albums");
    }
}
