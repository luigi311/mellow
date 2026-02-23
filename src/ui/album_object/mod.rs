use adw::subclass::prelude::*;
use glib::{Object, object::ObjectExt};
use gtk::{gdk, glib};
use std::cmp;
use std::sync::{Arc, atomic};

use crate::excuses::EXP_INIT;
use crate::library::album::SharedAlbum;
use crate::library::song::SharedSongExt;
use crate::library::{LIBRARY_TX, Library, song::SharedSong};
use crate::ui::{SortConfig, UI_TX, UpdateUI};

mod imp;

glib::wrapper! {
    pub struct AlbumObject(ObjectSubclass<imp::AlbumObject>);
}

impl AlbumObject {
    pub fn new(index: u32, album: &str, artist: &str, year: u32, first_song: SharedSong) -> Self {
        let album_object: AlbumObject = Object::builder()
            .property("index", index)
            .property("album", album)
            .property("artist", artist)
            .property("year", year)
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
        is_visible.store(true, atomic::Ordering::Relaxed);
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            if !is_visible.load(atomic::Ordering::Relaxed) {
                return;
            }
            drop(song.info().load_detailed());
            let ui_tx = UI_TX.get().expect(EXP_INIT);
            let _ = ui_tx.send(UpdateUI::LibraryAlbumLoaded(index));
        });
    }

    pub fn unload_artwork(&self) {
        self.set_property("artwork", Option::<gdk::Texture>::None);
        let imp = self.imp();
        let song = Arc::clone(imp.first_song.get().expect(EXP_INIT));
        let is_visible = Arc::clone(&imp.is_visible);
        is_visible.store(false, atomic::Ordering::Relaxed);
        // NOTE: Unloading in the background in case the `RwLock` is busy
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            if is_visible.load(atomic::Ordering::Relaxed) {
                return;
            }
            song.info().unload_detailed();
        });
    }

    pub fn shared_album(&self) -> SharedAlbum {
        self.imp()
            .first_song
            .get()
            .expect(EXP_INIT)
            .album()
            .clone()
            .expect(EXP_INIT)
    }

    #[inline]
    pub fn order_cmp(&self, other: &Self, sort_by: SortConfig<AlbumOrdering>) -> gtk::Ordering {
        let ord = match other.rank().total_cmp(&self.rank()) {
            cmp::Ordering::Equal => match sort_by.ordering.get() {
                AlbumOrdering::ArtistYearAlbum => self.cmp_artist_year_album(other),
                AlbumOrdering::ReleaseDate => self.cmp_release_date(other),
                AlbumOrdering::Modified => self.cmp_modified_newer(other),
                AlbumOrdering::Added => self.cmp_added_newer(other),
                AlbumOrdering::PlayCount => self.cmp_most_played(other),
                AlbumOrdering::Rating => self.cmp_best_rating(other),
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
    pub fn cmp_artist_year_album(&self, other: &Self) -> cmp::Ordering {
        match self.artist().cmp(&other.artist()) {
            cmp::Ordering::Equal => match self.year().cmp(&other.year()) {
                cmp::Ordering::Equal => self.album().cmp(&other.album()),
                ordering => ordering,
            },
            ordering => ordering,
        }
    }
    #[inline]
    fn cmp_most_played(&self, other: &Self) -> cmp::Ordering {
        println!("TODO: Sorting by average play count is not yet implemented");
        cmp::Ordering::Equal
    }
    #[inline]
    fn cmp_best_rating(&self, other: &Self) -> cmp::Ordering {
        println!("TODO: Sorting by best rating is not yet implemented");
        cmp::Ordering::Equal
    }
    #[inline]
    fn cmp_release_date(&self, other: &Self) -> cmp::Ordering {
        match other.year().cmp(&self.year()) {
            cmp::Ordering::Equal => self.cmp_artist_year_album(other),
            ordering => ordering,
        }
    }
    #[inline]
    fn cmp_modified_newer(&self, other: &Self) -> cmp::Ordering {
        // NOTE: Comparing modification time using the first song is not necessarily correct
        let modified_a = self.shared_album().lock().unwrap().songs[0]
            .info()
            .user()
            .modified;
        let modified_b = other.shared_album().lock().unwrap().songs[0]
            .info()
            .user()
            .modified;
        match modified_b.cmp(&modified_a) {
            cmp::Ordering::Equal => self.cmp_artist_year_album(other),
            ordering => ordering,
        }
    }
    #[inline]
    fn cmp_added_newer(&self, other: &Self) -> cmp::Ordering {
        let added_a = self.shared_album().lock().unwrap().songs[0]
            .info()
            .user()
            .added;
        let added_b = other.shared_album().lock().unwrap().songs[0]
            .info()
            .user()
            .added;
        match added_b.cmp(&added_a) {
            cmp::Ordering::Equal => self.cmp_artist_year_album(other),
            ordering => ordering,
        }
    }
}

#[derive(Default)]
pub struct AlbumData {
    index: u32,
    album: String,
    artist: String,
    artwork: Option<gdk::Texture>,
    year: u32,
    rank: f64,
}

#[derive(Clone, Copy)]
pub enum AlbumOrdering {
    ArtistYearAlbum,
    ReleaseDate,
    Modified,
    Added,
    Rating,
    PlayCount,
}
