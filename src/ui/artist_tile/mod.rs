use adw::subclass::prelude::*;
use glib::{Object, object::IsA};
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct ArtistTile(ObjectSubclass<imp::ArtistTile>)
        @extends gtk::Box, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for ArtistTile {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ArtistTile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> ArtistTileBuilder {
        ArtistTileBuilder {
            artist_tile: Self::default(),
        }
    }

    pub fn set_artwork(&self, artwork: &impl IsA<gdk::Paintable>) {
        self.imp().artist_image.set_paintable(Some(artwork));
    }

    pub fn set_info(&self, artist: &str, num_albums: u64) {
        let artist_tile = self.imp();
        artist_tile.artist.set_label(artist);
        artist_tile
            .num_albums
            .set_label(&format!("{num_albums} Albums"));
    }
}

pub struct ArtistTileBuilder {
    artist_tile: ArtistTile,
}

impl ArtistTileBuilder {
    pub fn artwork(self, artwork: &impl IsA<gdk::Paintable>) -> Self {
        // IDEA: Create an image using four of the artist's album covers
        self.artist_tile.set_artwork(artwork);
        self
    }

    pub fn info(&self, artist: &str, num_albums: u64) -> &Self {
        self.artist_tile.set_info(artist, num_albums);
        self
    }

    pub fn build(self) -> ArtistTile {
        self.artist_tile
    }
}
