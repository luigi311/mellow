use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::{gdk, glib};
use std::sync::Arc;

use crate::{excuses::EXP_INIT, library::artist::SharedArtist};

mod imp;

glib::wrapper! {
    pub struct ArtistObject(ObjectSubclass<imp::ArtistObject>);
}

impl ArtistObject {
    pub fn new(index: u32, artist: &str, albums: u64, shared_artist: SharedArtist) -> Self {
        let artist_object: ArtistObject = Object::builder()
            .property("index", index)
            .property("artist", artist)
            .property("albums", albums)
            .build();
        let _ = artist_object.imp().shared_artist.set(shared_artist);
        artist_object
    }

    pub fn load_artwork(&self) {
        // TODO: Decide what kind of image to show for library artists and construct it
        // Maybe 4 artworks composed in a grid with a circular cutout might look good
        // if self.artwork().is_some() {
        //     return;
        // }
        // let index = self.index() as usize;
        // Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
        //     UI_TX
        //         .get()
        //         .expect(EXP_INIT)
        //         .send(UpdateUI::LibraryArtistLoaded(index))
        //         .expect(EXP_RX);
        // });
    }

    pub fn unload_artwork(&self) {
        self.set_property("artwork", Option::<gdk::Texture>::None);
    }

    pub fn shared_artist(&self) -> SharedArtist {
        Arc::clone(self.imp().shared_artist.get().expect(EXP_INIT))
    }
}

#[derive(Default)]
pub struct ArtistData {
    index: u32,
    artist: String,
    albums: u64,
    artwork: Option<gdk::Paintable>,
    rank: f64,
}
