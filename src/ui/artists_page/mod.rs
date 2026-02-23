use adw::subclass::prelude::*;
use gtk::{gdk, glib};
use std::sync::{RwLock, atomic::AtomicBool};

use crate::{library::Artists, ui::artist_object::ArtistOrdering};

mod imp;

pub static ARTIST_ORDERING: RwLock<ArtistOrdering> = RwLock::new(ArtistOrdering::Artist);
pub static ARTISTS_REVERSE_ORDER: AtomicBool = AtomicBool::new(false);

glib::wrapper! {
    pub struct ArtistsPage(ObjectSubclass<imp::ArtistsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl ArtistsPage {
    #[inline]
    pub fn init_search(&self) {
        self.imp().init_search();
    }

    #[inline]
    pub fn load_artists(&self, artists: &Artists) {
        self.imp().load_artists(artists);
    }

    #[inline]
    pub fn assign_artwork(&self, index: u32, artwork: Option<gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    #[inline]
    pub fn set_sort_mode(&self, sort_mode: ArtistOrdering) {
        self.imp().set_sort_mode(sort_mode);
    }

    #[inline]
    pub fn uninit(&self) {
        self.imp().uninit();
    }
}
