use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::{gdk, glib};
use std::cmp;
use std::sync::Arc;

use crate::{excuses::EXP_INIT, library::artist::SharedArtist};

mod imp;

glib::wrapper! {
    pub struct ArtistObject(ObjectSubclass<imp::ArtistObject>);
}

impl ArtistObject {
    pub fn new(index: u32, artist: &str, albums: u64, shared_artist: SharedArtist) -> Self {
        let artist_object: ArtistObject = Object::builder()
            .property("index", index)
            .property("artist", artist)
            .property("albums", albums)
            .build();
        let _ = artist_object.imp().shared_artist.set(shared_artist);
        artist_object
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

    pub fn shared_artist(&self) -> SharedArtist {
        Arc::clone(self.imp().shared_artist.get().expect(EXP_INIT))
    }

    #[inline]
    pub fn order_cmp(&self, other: &Self, order_by: ArtistOrdering) -> gtk::Ordering {
        match other.rank().total_cmp(&self.rank()) {
            cmp::Ordering::Equal => match order_by {
                ArtistOrdering::Artist => self.cmp_artist(other),
                ArtistOrdering::AddedNewer => self.cmp_added_newer(other),
                ArtistOrdering::ModifiedNewer => self.cmp_modified_newer(other),
            },
            ordering => ordering,
        }
        .into()
    }
    #[inline]
    fn cmp_artist(&self, other: &Self) -> cmp::Ordering {
        self.artist().cmp(&other.artist())
    }
    #[inline]
    fn cmp_added_newer(&self, other: &Self) -> cmp::Ordering {
        // NOTE: Comparing added time using the oldest
        // album's first song is not necessarily correct
        let added_a = self.shared_artist().lock().unwrap().albums[0]
            .lock()
            .unwrap()
            .songs[0]
            .info()
            .user()
            .added;
        let added_b = other.shared_artist().lock().unwrap().albums[0]
            .lock()
            .unwrap()
            .songs[0]
            .info()
            .user()
            .added;
        match added_b.cmp(&added_a) {
            cmp::Ordering::Equal => self.cmp_artist(other),
            ordering => ordering,
        }
    }
    #[inline]
    fn cmp_modified_newer(&self, other: &Self) -> cmp::Ordering {
        // NOTE: Comparing added time using the newest
        // album's first song is not necessarily correct
        let modified_a = self
            .shared_artist()
            .lock()
            .unwrap()
            .albums
            .last()
            .unwrap()
            .lock()
            .unwrap()
            .songs[0]
            .info()
            .user()
            .modified;
        let modified_b = other
            .shared_artist()
            .lock()
            .unwrap()
            .albums
            .last()
            .unwrap()
            .lock()
            .unwrap()
            .songs[0]
            .info()
            .user()
            .modified;
        match modified_b.cmp(&modified_a) {
            cmp::Ordering::Equal => self.cmp_artist(other),
            ordering => ordering,
        }
    }
}

#[derive(Default)]
pub struct ArtistData {
    index: u32,
    artist: String,
    albums: u64,
    artwork: Option<gdk::Paintable>,
    rank: f64,
}

pub enum ArtistOrdering {
    // IDEA: Sort by average play count
    // IDEA: Sort by best average rating
    Artist,
    AddedNewer,
    ModifiedNewer,
}
