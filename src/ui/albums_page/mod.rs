use adw::subclass::prelude::*;
use gtk::{gdk, glib};
use std::sync::{RwLock, atomic::AtomicBool};

use crate::{library::Albums, ui::album_object::AlbumOrdering};

mod imp;

pub static ALBUM_ORDERING: RwLock<AlbumOrdering> = RwLock::new(AlbumOrdering::ArtistYearAlbum);
pub static ALBUMS_REVERSE_ORDER: AtomicBool = AtomicBool::new(false);

glib::wrapper! {
    pub struct AlbumsPage(ObjectSubclass<imp::AlbumsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl AlbumsPage {
    #[inline]
    pub fn init_search(&self) {
        self.imp().init_search();
    }

    #[inline]
    pub fn load_albums(&self, albums: &Albums) {
        self.imp().load_albums(albums);
    }

    #[inline]
    pub fn assign_artwork(&self, index: u32, artwork: Option<&gdk::Texture>) {
        self.imp().assign_artwork(index, artwork);
    }

    #[inline]
    pub fn set_sort_mode(&self, sort_mode: AlbumOrdering) {
        self.imp().set_sort_mode(sort_mode);
    }

    #[inline]
    pub fn uninit(&self) {
        self.imp().uninit();
    }
}
