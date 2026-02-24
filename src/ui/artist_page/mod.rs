use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::artist::SharedArtist;
use crate::ui::album_row::AlbumRow;
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
    #[inline]
    pub fn new(artist: &SharedArtist) -> ArtistPage {
        let artist_page = Self::default();
        artist_page.update(artist);
        artist_page
    }
    #[inline]
    pub fn update(&self, artist: &SharedArtist) {
        let ui = self.imp();
        ui.artist.replace(Some(Arc::clone(artist)));
        let artist = artist.lock().unwrap();
        self.set_title(&["Artist: ", &artist.name].concat());
        ui.artist_name.set_label(&artist.name);

        ui.albums_list.remove_all();
        for album in &artist.albums {
            let entry = AlbumRow::new();

            let album_locked = album.lock().unwrap();
            entry.set_title(&album_locked.title);
            entry.set_subtitle(&match album_locked.year {
                year if year > 0 => year.to_string(),
                _ => String::new(),
            });

            let mut info = album_locked.songs[0].info();
            let info = info.load_detailed();
            // SAFETY: `load_detailed` ensures the value is `Some`
            let artwork = unsafe { info.as_ref().unwrap_unchecked().artwork.as_ref() };
            if artwork.is_some() {
                entry.set_prefix_image(artwork);
            } else {
                entry.set_prefix_image(Some(&fallback_song_image()));
            }

            let album = Arc::clone(album);
            entry.connect_activated(move |_| {
                UI_TX
                    .get()
                    .expect(EXP_INIT)
                    .send(UpdateUI::AlbumPage(Arc::clone(&album)))
                    .expect(EXP_RX);
            });

            ui.albums_list.append(&entry);
        }
    }
    #[inline]
    pub fn set_shuffle(&self, shuffle: bool) {
        self.imp().set_shuffle(shuffle);
    }
}
