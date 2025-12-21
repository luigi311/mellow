use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use std::sync::mpsc;

use crate::excuses::INIT_ERR;
use crate::library::{LibraryRequest, Songs};
use crate::player::PlayerRequest;

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

    pub fn init(
        &self,
        library_tx: mpsc::SyncSender<LibraryRequest>,
        player_tx: mpsc::SyncSender<PlayerRequest>,
    ) {
        let songs_page = self.imp();
        songs_page.library_tx.set(library_tx).expect(INIT_ERR);
        songs_page.player_tx.set(player_tx).expect(INIT_ERR);
    }

    pub fn load_songs(&self, songs: &Songs) {
        println!("load_songs()");
        self.imp().load_songs(songs);
    }
}
