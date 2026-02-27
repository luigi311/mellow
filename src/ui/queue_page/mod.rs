use adw::{prelude::*, subclass::prelude::*};
use gtk::{gdk, glib};

use crate::player::QueueItem;
use crate::ui::QueueSubpage;

mod imp;

glib::wrapper! {
    pub struct QueuePage(ObjectSubclass<imp::QueuePage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl QueuePage {
    #[inline]
    pub fn init(&self, song_page: QueueSubpage) {
        let _ = self.imp().song_page.set(song_page);
    }

    #[inline]
    #[must_use]
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

    #[inline]
    #[must_use]
    pub fn get_repeat(&self) -> bool {
        self.imp().repeat_toggle.is_active()
    }
    #[inline]
    pub fn update_repeat(&self, repeat: bool) {
        self.imp().repeat_toggle.set_active(repeat);
    }

    #[inline]
    pub fn update_song_queue(&self, queue: &[QueueItem], index: usize) {
        self.imp().update_song_queue(queue, index);
    }
    #[inline]
    pub fn assign_artwork(&self, index: usize, artwork: Option<&gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    /// Empties the list model, cancelling any pending background tasks during drop
    #[inline]
    pub fn uninit(&self) {
        self.imp().uninit();
    }
}
