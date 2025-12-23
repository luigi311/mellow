use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use std::sync::mpsc;

use crate::excuses::INIT_ERR;
use crate::library::{Albums, LibraryRequest};
use crate::player::PlayerRequest;

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

    pub fn init(
        &self,
        library_tx: mpsc::Sender<LibraryRequest>,
        player_tx: mpsc::Sender<PlayerRequest>,
    ) {
        let albums_page = self.imp();
        albums_page.library_tx.set(library_tx).expect(INIT_ERR);
        albums_page.player_tx.set(player_tx).expect(INIT_ERR);
    }

    pub fn load_albums(&self, albums: &Albums) {
        println!("load_albums()");
        self.imp().load_albums(albums);
    }
}
