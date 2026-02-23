use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::{gdk, glib};
use std::cmp;
use std::sync::{Arc, atomic};

use crate::excuses::EXP_INIT;
use crate::library::{LIBRARY_TX, Library, song::SharedSong};
use crate::ui::{SortConfig, UI_TX, UpdateUI};

mod imp;

glib::wrapper! {
    pub struct SongObject(ObjectSubclass<imp::SongObject>);
}

impl SongObject {
    pub fn new(index: u32, song: SharedSong) -> Self {
        let (title, album, artist, year) = {
            let mut info = song.info();
            let info_temp = info.load_basic();
            // SAFETY: `load_basic` ensures the value is `Some`
            let info = unsafe { info_temp.as_ref().unwrap_unchecked() };
            (
                info.title.clone(),
                info.artist.clone(),
                info.artist.clone(),
                info.year as u32,
            )
        };
        let song_object: SongObject = Object::builder()
            .property("index", index)
            .property("song", title)
            .property("album", album)
            .property("artist", artist)
            .property("year", year)
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
        is_visible.store(true, atomic::Ordering::Relaxed);
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            if !is_visible.load(atomic::Ordering::Relaxed) {
                return;
            }
            drop(song.info().load_detailed());
            let ui_tx = UI_TX.get().expect(EXP_INIT);
            let _ = ui_tx.send(UpdateUI::LibrarySongLoaded(index));
        });
    }

    pub fn unload_artwork(&self) {
        self.set_property("artwork", Option::<gdk::Texture>::None);
        let song = Arc::clone(self.imp().shared_song.get().expect(EXP_INIT));
        let is_visible = Arc::clone(&self.imp().is_visible);
        is_visible.store(false, atomic::Ordering::Relaxed);
        // NOTE: Unloading in the background in case the `RwLock` is busy
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            if is_visible.load(atomic::Ordering::Relaxed) {
                return;
            }
            song.info().try_unload_detailed();
        });
    }

    pub fn shared_song(&self) -> SharedSong {
        Arc::clone(self.imp().shared_song.get().expect(EXP_INIT))
    }

    #[inline]
    pub fn order_cmp(&self, other: &Self, sort_by: SortConfig<SongOrdering>) -> gtk::Ordering {
        let ord = match other.rank().total_cmp(&self.rank()) {
            cmp::Ordering::Equal => match sort_by.ordering.get() {
                SongOrdering::Default => self.cmp_default(other),
                SongOrdering::BestRating => self.cmp_best_rating(other),
                SongOrdering::MostPlayed => self.cmp_most_played(other),
                SongOrdering::AddedNewer => self.cmp_added_newer(other),
                SongOrdering::ModifiedNewer => self.cmp_modified_newer(other),
            },
            ordering => ordering,
        };
        match sort_by.reversed.get() {
            false => ord,
            true => ord.reverse(),
        }
        .into()
    }
    #[inline]
    fn cmp_default(&self, other: &Self) -> cmp::Ordering {
        match self.artist().cmp(&other.artist()) {
            cmp::Ordering::Equal => match self.year().cmp(&other.year()) {
                cmp::Ordering::Equal => match self.album().cmp(&other.album()) {
                    cmp::Ordering::Equal => self.song().cmp(&other.song()),
                    ordering => ordering,
                },
                ordering => ordering,
            },
            ordering => ordering,
        }
    }
    #[inline]
    fn cmp_best_rating(&self, other: &Self) -> cmp::Ordering {
        // TODO: Add `rating` to `SongObject` (and update so it stays in sync)
        let rating_a = self.shared_song().info().user().rating;
        let rating_b = other.shared_song().info().user().rating;
        match rating_b.cmp(&rating_a) {
            cmp::Ordering::Equal => self.cmp_default(other),
            ordering => ordering,
        }
    }
    #[inline]
    fn cmp_most_played(&self, other: &Self) -> cmp::Ordering {
        // TODO: Add `play_count` to `SongObject` (and update so it stays in sync)
        let play_count_a = self.shared_song().info().user().play_count;
        let play_count_b = other.shared_song().info().user().play_count;
        match play_count_b.cmp(&play_count_a) {
            cmp::Ordering::Equal => self.cmp_default(other),
            ordering => ordering,
        }
    }
    #[inline]
    fn cmp_added_newer(&self, other: &Self) -> cmp::Ordering {
        // TODO: Add `added` to `SongObject`
        let added_a = self.shared_song().info().user().added;
        let added_b = other.shared_song().info().user().added;
        match added_b.cmp(&added_a) {
            cmp::Ordering::Equal => self.cmp_default(other),
            ordering => ordering,
        }
    }
    #[inline]
    fn cmp_modified_newer(&self, other: &Self) -> cmp::Ordering {
        // TODO: Add `modified` to `SongObject`
        let modified_a = self.shared_song().info().user().modified;
        let modified_b = other.shared_song().info().user().modified;
        match modified_b.cmp(&modified_a) {
            cmp::Ordering::Equal => self.cmp_default(other),
            ordering => ordering,
        }
    }
}

#[derive(Default)]
pub struct SongData {
    index: u32,
    song: String,
    album: String,
    artist: String,
    artwork: Option<gdk::Texture>,
    year: u32,
    rank: f64,
}

#[derive(Clone, Copy)]
pub enum SongOrdering {
    Default,
    BestRating,
    MostPlayed,
    AddedNewer,
    ModifiedNewer,
}
