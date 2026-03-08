use adw::subclass::prelude::*;
use core::sync::atomic::{AtomicBool, Ordering};
use glib::Object;
use gtk::{gdk, glib};
use std::sync::Arc;

use crate::excuses::EXP_INIT;
use crate::library::{LIBRARY_TX, Library, SharedSong};
use crate::ui::{UI_TX, UpdateUI};

mod imp;

glib::wrapper! {
    pub struct QueueItemObject(ObjectSubclass<imp::QueueItemObject>);
}

impl QueueItemObject {
    #[must_use]
    pub fn new(index: u32, playing: bool, song: Option<SharedSong>) -> Self {
        let song_object: QueueItemObject = Object::builder()
            .property("index", index)
            .property("playing", playing)
            .build();
        let _ = song_object.imp().shared_song.set(song);
        song_object
    }

    /// Loads the artwork thumbnail in a background thread
    ///
    /// # Panics
    /// The function panics if `shared_song`, `LIBRARY_TX`,
    /// or `UI_TX` is uninitialized
    pub fn load_artwork(&self) {
        if self.artwork().is_some() {
            return;
        }
        let index = self.index() as usize;
        let song = self.imp().shared_song.get().expect(EXP_INIT).clone();
        let is_visible = Arc::clone(&self.imp().is_visible);
        is_visible.store(true, Ordering::Relaxed);
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            if !is_visible.load(Ordering::Relaxed) {
                return;
            }
            let Some(song) = song else {
                return;
            };
            drop(song.info().load_thumbnail());
            let ui_tx = UI_TX.get().expect(EXP_INIT);
            let _ = ui_tx.send(UpdateUI::QueueSongLoaded(index));
        });
    }

    /// Returns the `SharedSong` associated with this object
    ///
    /// # Panics
    /// The function panics if `shared_song` is uninitialized
    #[must_use]
    pub fn shared_song(&self) -> Option<&SharedSong> {
        self.imp().shared_song.get().expect(EXP_INIT).as_ref()
    }

    /// Returns `true` if the item is currently shown in the UI,
    /// otherwise it returns `false`
    #[must_use]
    pub fn is_visible(&self) -> &Arc<AtomicBool> {
        &self.imp().is_visible
    }
}

#[derive(Default)]
pub struct QueueItemData {
    index: u32,
    playing: bool,
    title: String,
    subtitle: String,
    suffix: String,
    artwork: Option<gdk::Texture>,
}
