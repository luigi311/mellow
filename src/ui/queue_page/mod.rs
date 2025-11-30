use adw::subclass::prelude::*;
use glib::Object;
use gtk::{glib, prelude::*};
use std::cell::Ref;
use std::sync::mpsc;

use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::song_page::SongPage;

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

    pub fn init(
        &self,
        player_tx: mpsc::SyncSender<PlayerRequest>,
        song_page: SongPage,
        navigation_view: adw::NavigationView,
    ) {
        let queue_page = self.imp();
        queue_page.player_tx.set(player_tx).unwrap();
        queue_page.song_page.set(song_page).unwrap();
        queue_page.navigation_view.set(navigation_view).unwrap();
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

    pub fn update_song_queue(&self, queue: Ref<'_, Box<[QueueItem]>>, index: usize) {
        self.imp().update_song_queue(queue, index);
    }
}
