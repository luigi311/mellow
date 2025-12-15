use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;
use std::sync::mpsc;

use crate::excuses::INIT_ERR;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::queue_song_page::QueueSongPage;

mod imp;

glib::wrapper! {
    pub struct QueuePage(ObjectSubclass<imp::QueuePage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for QueuePage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl QueuePage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn init(&self, player_tx: mpsc::SyncSender<PlayerRequest>, song_page: QueueSongPage) {
        let queue_page = self.imp();
        queue_page.player_tx.set(player_tx).expect(INIT_ERR);
        queue_page.song_page.set(song_page).expect(INIT_ERR);
    }

    pub fn update_shuffle(&self, shuffle: bool) {
        let ui = self.imp();
        ui.shuffle_toggle.set_icon_name(match shuffle {
            true => "media-playlist-shuffle-symbolic",
            false => "media-playlist-consecutive-symbolic",
        });
        ui.shuffle_toggle.set_active(shuffle);
    }

    pub fn update_repeat(&self, repeat: bool) {
        self.imp().repeat_toggle.set_active(repeat);
    }

    pub fn update_song_queue(&self, queue: &[QueueItem], index: usize) {
        self.imp().update_song_queue(queue, index);
    }
}
