use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use std::sync::mpsc;

use crate::excuses::INIT_ERR;
use crate::player::PlayerRequest;

mod imp;

glib::wrapper! {
    pub struct QueueSongPage(ObjectSubclass<imp::QueueSongPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for QueueSongPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl QueueSongPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_info(&self, index: usize, song_title: &str, album_title: &str, artist_name: &str) {
        let song_page = self.imp();
        song_page.index.set(index);
        song_page.song_title.set_label(song_title);
        song_page.album_title.set_label(album_title);
        song_page.artist_name.set_label(artist_name);
    }

    pub fn init(&self, player_tx: mpsc::SyncSender<PlayerRequest>) {
        let song_page = self.imp();
        song_page.player_tx.set(player_tx).expect(INIT_ERR);
    }
}
