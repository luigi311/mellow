use adw::subclass::prelude::*;
use gtk::{gdk, glib};
use std::sync::{RwLock, atomic::AtomicBool};

use crate::{library::Songs, ui::song_object::SongOrdering};

mod imp;

pub static SONG_ORDERING: RwLock<SongOrdering> = RwLock::new(SongOrdering::Default);
pub static SONGS_REVERSE_ORDER: AtomicBool = AtomicBool::new(false);

glib::wrapper! {
    pub struct SongsPage(ObjectSubclass<imp::SongsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl SongsPage {
    #[inline]
    pub fn init_search(&self) {
        self.imp().init_search();
    }

    #[inline]
    pub fn load_songs(&self, songs: &Songs) {
        self.imp().load_songs(songs);
    }

    #[inline]
    pub fn assign_artwork(&self, index: u32, artwork: Option<&gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    #[inline]
    pub fn set_sort_mode(&self, sort_mode: SongOrdering) {
        self.imp().set_sort_mode(sort_mode);
    }

    #[inline]
    pub fn uninit(&self) {
        self.imp().uninit();
    }
}
