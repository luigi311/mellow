use adw::subclass::prelude::*;
use glib::Object;
use gtk::{gdk, glib};

use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::excuses::EXP_INIT;
use crate::library::{LIBRARY_TX, Library, song::SharedSong};
use crate::ui::{UI_TX, UpdateUI};

mod imp;

glib::wrapper! {
    pub struct QueueItemObject(ObjectSubclass<imp::QueueItemObject>);
}

impl QueueItemObject {
    pub fn new(
        index: u32,
        playing: bool,
        title: String,
        subtitle: String,
        song: Option<SharedSong>,
    ) -> Self {
        let song_object: QueueItemObject = Object::builder()
            .property("index", index)
            .property("playing", playing)
            .property("title", title)
            .property("subtitle", subtitle)
            .build();
        let _ = song_object.imp().shared_song.set(song);
        song_object
    }

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
            drop(song.info().load_detailed());
            // TODO: Update the queue_object artwork, same as song_/album_object
            // let ui_tx = UI_TX.get().expect(EXP_INIT);
            // let _ = ui_tx.send(UpdateUI::QueueSongLoaded(index));
        });
    }

    pub fn shared_song(&self) -> Option<&SharedSong> {
        self.imp().shared_song.get().expect(EXP_INIT).as_ref()
    }
}

#[derive(Default)]
pub struct QueueItemData {
    index: u32,
    playing: bool,
    title: String,
    subtitle: String,
    artwork: Option<gdk::Texture>,
}
