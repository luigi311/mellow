use adw::subclass::prelude::*;
use gtk::{gdk, glib};

use crate::library::Songs;
use crate::ui::{SortConfig, song_object::SongOrdering};

mod imp;

glib::wrapper! {
    pub struct SongsPage(ObjectSubclass<imp::SongsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl SongsPage {
    #[inline]
    pub fn load_songs(&self, songs: &Songs) {
        self.imp().load_songs(songs);
    }

    #[inline]
    pub fn assign_artwork(&self, index: usize, artwork: Option<&gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    #[inline]
    pub fn set_sort_mode(&self, sort_mode: SongOrdering) {
        self.imp().set_sort_mode(sort_mode);
    }
    #[inline]
    pub fn get_sort_config(&self) -> &SortConfig<SongOrdering> {
        self.imp().get_sort_mode()
    }

    #[inline]
    pub fn set_shuffle(&self, shuffle: bool) {
        self.imp().set_shuffle(shuffle);
    }
    #[inline]
    pub fn get_shuffle(&self) -> bool {
        self.imp().get_shuffle()
    }

    #[inline]
    pub fn uninit(&self) {
        self.imp().uninit();
    }
}
