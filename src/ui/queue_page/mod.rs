use adw::{prelude::*, subclass::prelude::*};
use gtk::{gdk, glib};

use crate::player::queue_item::QueueItem;
use crate::ui::queue_subpage::QueueSubpage;

mod imp;

glib::wrapper! {
    pub struct QueuePage(ObjectSubclass<imp::QueuePage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl QueuePage {
    pub fn init(&self, song_page: QueueSubpage) {
        let queue_page = self.imp();
        let _ = queue_page.song_page.set(song_page);
    }

    pub fn get_shuffle(&self) -> bool {
        self.imp().shuffle_toggle.is_active()
    }
    pub fn update_shuffle(&self, shuffle: bool) {
        let ui = self.imp();
        ui.shuffle_toggle.set_icon_name(match shuffle {
            true => "media-playlist-shuffle-symbolic",
            false => "media-playlist-consecutive-symbolic",
        });
        ui.shuffle_toggle.set_active(shuffle);
    }

    pub fn get_repeat(&self) -> bool {
        self.imp().repeat_toggle.is_active()
    }
    pub fn update_repeat(&self, repeat: bool) {
        self.imp().repeat_toggle.set_active(repeat);
    }

    pub fn update_song_queue(&self, queue: &[QueueItem], index: usize) {
        self.imp().update_song_queue(queue, index);
    }
    pub fn assign_artwork(&self, index: usize, artwork: Option<&gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    pub fn uninit(&self) {
        self.imp().uninit();
    }
}
