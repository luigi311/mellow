use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib;
use std::sync::mpsc;

use crate::player::PlayerRequest;

use crate::excuses::INIT_ERR;

mod imp;

glib::wrapper! {
    pub struct SongPage(ObjectSubclass<imp::SongPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for SongPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl SongPage {
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

    pub fn init(
        &self,
        player_tx: mpsc::SyncSender<PlayerRequest>,
        navigation: adw::NavigationView,
        bottom_sheet: adw::BottomSheet,
    ) {
        let song_page = self.imp();
        song_page.player_tx.set(player_tx).expect(INIT_ERR);
        song_page.navigation_view.set(navigation).expect(INIT_ERR);
        song_page.bottom_sheet.set(bottom_sheet).expect(INIT_ERR);
    }
}
