use std::sync::Arc;

use adw::subclass::prelude::*;
use glib::Object;
use gtk::{gdk, glib};

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, Library, song::SharedSong};
use crate::ui::{UI_TX, UpdateUI};

mod imp;

glib::wrapper! {
    pub struct AlbumObject(ObjectSubclass<imp::AlbumObject>);
}

impl AlbumObject {
    pub fn new(index: u32, album: &str, artist: &str, first_song: SharedSong) -> Self {
        let album_object: AlbumObject = Object::builder()
            .property("index", index)
            .property("album", album)
            .property("artist", artist)
            .property(
                "artwork",
                first_song
                    .lock()
                    .unwrap()
                    .info()
                    .inspect_detailed()
                    .and_then(|info| info.artwork.clone()),
            )
            .build();
        let _ = album_object.imp().first_song.set(first_song);
        album_object
    }

    pub fn load_artwork(&self) {
        let index = self.index() as usize;
        let song = Arc::clone(self.imp().first_song.get().expect(EXP_INIT));
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            println!("Loading artwork");
            song.lock().expect(EXP_INIT).info().load_detailed();
            UI_TX
                .get()
                .expect(EXP_INIT)
                .send(UpdateUI::LibraryAlbumLoaded(index))
                .expect(EXP_RX);
        });
    }

    pub fn unload_artwork(&self) {
        Arc::clone(self.imp().first_song.get().expect(EXP_INIT))
            .lock()
            .unwrap()
            .info()
            .unload_detailed();
        // TODO: Unassign artwork from `self`
    }
}

#[derive(Default)]
pub struct AlbumData {
    index: u32,
    album: String,
    artist: String,
    artwork: Option<gdk::Texture>,
}
