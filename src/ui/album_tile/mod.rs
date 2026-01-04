use adw::subclass::prelude::*;
use glib::{Object, object::IsA};
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct AlbumTile(ObjectSubclass<imp::AlbumTile>)
        @extends gtk::Box, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for AlbumTile {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl AlbumTile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_artwork(&self, artwork: &impl IsA<gdk::Paintable>) {
        self.imp().album_cover.set_paintable(Some(artwork));
    }

    pub fn set_info(&self, album: &str, artist: &str) {
        let album_tile = self.imp();
        album_tile.album.set_label(album);
        album_tile.artist.set_label(artist);
    }
}
