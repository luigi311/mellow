use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::{gdk, glib};

use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, Library, song::SharedSong};
use crate::ui::{UI_TX, UpdateUI};

mod imp;

glib::wrapper! {
    pub struct SongObject(ObjectSubclass<imp::SongObject>);
}

impl SongObject {
    pub fn new(index: u32, song: SharedSong) -> Self {
        let (title, artist) = {
            let mut info = song.info();
            let info_temp = info.load_basic();
            // SAFETY: `load_basic` is always safe to unwrap
            let info = unsafe { info_temp.as_ref().unwrap_unchecked() };
            (info.title.clone(), info.artist.clone())
        };
        let song_object: SongObject = Object::builder()
            .property("index", index)
            .property("song", title)
            .property("artist", artist)
            .build();
        let _ = song_object.imp().shared_song.set(song);
        song_object
    }

    pub fn load_artwork(&self) {
        if self.artwork().is_some() {
            return;
        }
        let index = self.index() as usize;
        let song = Arc::clone(self.imp().shared_song.get().expect(EXP_INIT));
        let is_visible = Arc::clone(&self.imp().is_visible);
        is_visible.store(true, Ordering::Relaxed);
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            if !is_visible.load(Ordering::Relaxed) {
                return;
            }
            drop(song.info().load_detailed());
            UI_TX
                .get()
                .expect(EXP_INIT)
                .send(UpdateUI::LibrarySongLoaded(index))
                .expect(EXP_RX);
        });
    }

    pub fn unload_artwork(&self) {
        self.set_property("artwork", Option::<gdk::Texture>::None);
        let song = Arc::clone(self.imp().shared_song.get().expect(EXP_INIT));
        let is_visible = Arc::clone(&self.imp().is_visible);
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
pub struct SongData {
    index: u32,
    song: String,
    artist: String,
    artwork: Option<gdk::Texture>,
}
