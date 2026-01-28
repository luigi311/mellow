use adw::subclass::prelude::*;
use glib::{Object, object::IsA};
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct SongTile(ObjectSubclass<imp::SongTile>)
        @extends gtk::Box, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for SongTile {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl SongTile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> SongTileBuilder {
        SongTileBuilder {
            album_tile: Self::default(),
        }
    }

    pub fn set_artwork(&self, artwork: &impl IsA<gdk::Paintable>) {
        self.imp().album_cover.set_paintable(Some(artwork));
    }

    pub fn set_info(&self, title: &str, artist: &str) {
        let album_tile = self.imp();
        album_tile.title.set_label(title);
        album_tile.artist.set_label(artist);
    }
}

pub struct SongTileBuilder {
    album_tile: SongTile,
}

impl SongTileBuilder {
    pub fn artwork(self, artwork: &impl IsA<gdk::Paintable>) -> Self {
        self.album_tile.set_artwork(artwork);
        self
    }

    pub fn info(self, album: &str, artist: &str) -> Self {
        self.album_tile.set_info(album, artist);
        self
    }

    pub fn build(self) -> SongTile {
        self.album_tile
    }
}
