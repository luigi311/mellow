use adw::subclass::prelude::*;
use glib::Object;
use gtk::{gdk, glib};

use crate::ui::fallback_album_image;

mod imp;

glib::wrapper! {
    pub struct AlbumPage(ObjectSubclass<imp::AlbumPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for AlbumPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl AlbumPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_info(
        &self,
        index: usize,
        album: &str,
        artist: &str,
        year: &str,
        artwork: Option<&gdk::Texture>,
    ) {
        let ui = self.imp();
        if artwork.is_some() {
            ui.album_cover.set_paintable(artwork);
        } else {
            ui.album_cover.set_paintable(Some(&fallback_album_image()));
        }
        ui.index.set(index);
        ui.album_title.set_label(album);
        ui.artist_name.set_label(artist);
        ui.year.set_label(year);
    }
}
