use adw::subclass::prelude::*;
use gtk::{gdk, glib};

use crate::library::Artists;
use crate::ui::{ArtistOrdering, SortConfig};

mod imp;

glib::wrapper! {
    pub struct ArtistsPage(ObjectSubclass<imp::ArtistsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl ArtistsPage {
    #[inline]
    pub fn load_artists(&self, artists: &Artists) {
        self.imp().load_artists(artists);
    }

    #[inline]
    pub fn assign_artwork(&self, index: usize, artwork: Option<gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    #[inline]
    pub fn set_sort_mode(&self, sort_mode: ArtistOrdering) {
        self.imp().set_sort_mode(sort_mode);
    }
    #[inline]
    pub fn get_sort_config(&self) -> &SortConfig<ArtistOrdering> {
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
