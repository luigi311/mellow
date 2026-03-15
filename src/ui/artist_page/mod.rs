use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::SharedArtist;
use crate::ui::ListRow;
use crate::ui::{UI_TX, UpdateUI, fallback_song_image};

mod imp;

glib::wrapper! {
    pub struct ArtistPage(ObjectSubclass<imp::ArtistPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for ArtistPage {
    #[inline]
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ArtistPage {
    /// Creates a new `ArtistPage` instance using the information from `artist`
    ///
    /// # Panics
    /// The function panics if any of the artist albums' `Mutex`es or songs'
    /// `RwLock`s are in a poisoned state. It may also panic at runtime upon
    /// interaction if `UI_TX` is uninitialized, or the channel is closed.
    #[inline]
    #[must_use]
    pub fn new(artist: &SharedArtist) -> ArtistPage {
        let artist_page = Self::default();
        let ui = artist_page.imp();

        ui.artist.replace(Some(Arc::clone(artist)));
        let artist = artist.lock().unwrap();
        artist_page.set_title(&["Artist: ", artist.name()].concat());
        ui.artist_name.set_label(artist.name());

        ui.albums_list.remove_all();
        for album in artist.albums() {
            let album_row = ListRow::new();

            let album_locked = album.lock().unwrap();
            album_row.set_title(album_locked.title());
            album_row.set_subtitle(&match album_locked.year() {
                year if year > 0 => year.to_string(),
                _ => String::new(),
            });

            let mut info = album_locked.songs()[0].info();
            let thumbnail = info.load_thumbnail();
            if thumbnail.is_some() {
                album_row.set_prefix_image(thumbnail.as_ref());
            } else {
                album_row.set_prefix_image(Some(&fallback_song_image()));
            }

            let album = Arc::clone(album);
            album_row.connect_activated(move |_| {
                (UI_TX.get().expect(EXP_INIT))
                    .send(UpdateUI::AlbumPage(Arc::clone(&album)))
                    .expect(EXP_RX);
            });

            ui.albums_list.append(&album_row);
        }

        artist_page
    }
    /// Sets the shuffle mode for the play button
    #[inline]
    pub fn set_shuffle(&self, shuffle: bool) {
        self.imp().set_shuffle(shuffle);
    }
}
