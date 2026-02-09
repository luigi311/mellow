use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::{gdk, glib};

use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::song::SharedSongExt;
use crate::library::{LIBRARY_TX, Library, song::SharedSong};
use crate::ui::{UI_TX, UpdateUI};

mod imp;

glib::wrapper! {
    pub struct SongObject(ObjectSubclass<imp::SongObject>);
}

impl SongObject {
    pub fn new(index: u32, song: SharedSong) -> Self {
        let mut song_locked = song.lock().unwrap();
        let mut info = song_locked.info();
        let info = info.basic();
        let song_object: SongObject = Object::builder()
            .property("index", index)
            .property("song", info.title.clone())
            .property("artist", info.artist.clone())
            .build();
        drop(song_locked);
        let _ = song_object.imp().first_song.set(song);
        song_object
    }

    pub fn load_artwork(&self) {
        if self.artwork().is_some() {
            return;
        }
        // TODO: Don't load if already loading
        let index = self.index() as usize;
        let song = Arc::clone(self.imp().first_song.get().expect(EXP_INIT));
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            // TODO: Load in a way that allows cancellation in `unbind`
            let _ = song.load_detailed_info();
            UI_TX
                .get()
                .expect(EXP_INIT)
                .send(UpdateUI::LibrarySongLoaded(index))
                .expect(EXP_RX);
        });
    }

    pub fn unload_artwork(&self) {
        // FIX: Info loading can't be cancelled, and can't be unloaded until done loading
        if let Ok(mut song) = self.imp().first_song.get().expect(EXP_INIT).try_lock() {
            self.set_property("artwork", Option::<gdk::Texture>::None);
            song.info().unload_detailed();
        }
    }
}

#[derive(Default)]
pub struct SongData {
    index: u32,
    song: String,
    artist: String,
    artwork: Option<gdk::Texture>,
}
