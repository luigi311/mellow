use std::sync::Arc;
use std::sync::atomic::Ordering;

use adw::subclass::prelude::*;
use glib::Object;
use gst::glib::object::ObjectExt;
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
            .build();
        let _ = album_object.imp().first_song.set(first_song);
        album_object
    }

    pub fn load_artwork(&self) {
        if self.artwork().is_some() {
            return;
        }
        let index = self.index() as usize;
        let imp = self.imp();
        let song = Arc::clone(imp.first_song.get().expect(EXP_INIT));
        let is_visible = Arc::clone(&imp.is_visible);
        is_visible.store(true, Ordering::Relaxed);
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            if !is_visible.load(Ordering::Relaxed) {
                return;
            }
            drop(song.info().load_detailed());
            UI_TX
                .get()
                .expect(EXP_INIT)
                .send(UpdateUI::LibraryAlbumLoaded(index))
                .expect(EXP_RX);
        });
    }

    pub fn unload_artwork(&self) {
        self.set_property("artwork", Option::<gdk::Texture>::None);
        let imp = self.imp();
        let song = Arc::clone(imp.first_song.get().expect(EXP_INIT));
        let is_visible = Arc::clone(&imp.is_visible);
        is_visible.store(false, Ordering::Relaxed);
        // NOTE: Unloading in the background in case the `RwLock` is busy
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            if is_visible.load(Ordering::Relaxed) {
                return;
            }
            song.info().unload_detailed();
        });
    }
}

#[derive(Default)]
pub struct AlbumData {
    index: u32,
    album: String,
    artist: String,
    artwork: Option<gdk::Texture>,
}
