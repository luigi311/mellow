use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::artist::ArtistMutex;
use crate::ui::album_row::AlbumRow;
use crate::ui::{UI_TX, UpdateUI, fallback_song_image};

mod imp;

glib::wrapper! {
    pub struct ArtistPage(ObjectSubclass<imp::ArtistPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl ArtistPage {
    pub fn update(&self, artist: &ArtistMutex) {
        let ui = self.imp();
        ui.artist.replace(Some(Arc::clone(artist)));
        let artist = artist.lock().unwrap();
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

            let mut first_song = album_locked.songs[0].lock().unwrap();
            let mut info = first_song.info();
            let info = info.detailed();
            let artwork = info.artwork.as_ref();
            if artwork.is_some() {
                entry.set_prefix_image(artwork);
            } else {
                entry.set_prefix_image(Some(&fallback_song_image()));
            }

            entry.connect_activated({
                let album = Arc::clone(album);
                move |_| {
                    UI_TX
                        .get()
                        .expect(EXP_INIT)
                        .send(UpdateUI::AlbumPage(Arc::clone(&album)))
                        .expect(EXP_RX);
                }
            });

            ui.albums_list.append(&entry);
        }
    }
}
