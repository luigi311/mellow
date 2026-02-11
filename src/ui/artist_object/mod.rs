use adw::prelude::*;
use glib::Object;
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct ArtistObject(ObjectSubclass<imp::ArtistObject>);
}

impl ArtistObject {
    pub fn new(artist: &str, albums: u64) -> Self {
        Object::builder()
            .property("artist", artist)
            .property("albums", albums)
            .build()
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
}

#[derive(Default)]
pub struct ArtistData {
    artist: String,
    albums: u64,
    artwork: Option<gdk::Paintable>,
}
