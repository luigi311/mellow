use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use std::sync::mpsc;

use crate::excuses::INIT_ERR;
use crate::library::{Artists, LibraryRequest};
use crate::player::PlayerRequest;

mod imp;

glib::wrapper! {
    pub struct LibraryArtistsPage(ObjectSubclass<imp::LibraryArtistsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for LibraryArtistsPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibraryArtistsPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn init(
        &self,
        library_tx: mpsc::SyncSender<LibraryRequest>,
        player_tx: mpsc::SyncSender<PlayerRequest>,
    ) {
        let artists_page = self.imp();
        artists_page.library_tx.set(library_tx).expect(INIT_ERR);
        artists_page.player_tx.set(player_tx).expect(INIT_ERR);
    }

    pub fn load_artists(&self, artists: &Artists) {
        println!("load_artists()");
        self.imp().load_artists(artists);
    }
}
