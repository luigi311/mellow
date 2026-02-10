use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::{gdk, glib};

use std::sync::Arc;

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
            let info = unsafe { info_temp.as_ref().unwrap_unchecked() };
            (info.title.clone(), info.artist.clone())
        };
        let song_object: SongObject = Object::builder()
            .property("index", index)
            .property("song", title)
            .property("artist", artist)
            .build();
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
            drop(song.info().load_detailed());
            UI_TX
                .get()
                .expect(EXP_INIT)
                .send(UpdateUI::LibrarySongLoaded(index))
                .expect(EXP_RX);
        });
    }

    pub fn unload_artwork(&self) {
        // FIX: Info loading can't be cancelled, and can't be unloaded until done loading
        self.set_property("artwork", Option::<gdk::Texture>::None);
        let song = self.imp().first_song.get().expect(EXP_INIT);
        song.info().unload_detailed();
    }
}

#[derive(Default)]
pub struct SongData {
    index: u32,
    song: String,
    artist: String,
    artwork: Option<gdk::Texture>,
}
