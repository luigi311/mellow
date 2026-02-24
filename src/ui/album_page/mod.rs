use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::format_duration_ms;
use crate::library::album::SharedAlbum;
use crate::ui::song_row::SongRow;
use crate::ui::{UI_TX, UpdateUI, fallback_album_image};

mod imp;

glib::wrapper! {
    pub struct AlbumPage(ObjectSubclass<imp::AlbumPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for AlbumPage {
    #[inline]
    fn default() -> Self {
        Object::builder().build()
    }
}

impl AlbumPage {
    #[inline]
    pub fn new(album: &SharedAlbum) -> AlbumPage {
        let album_page = Self::default();
        album_page.update(album);
        album_page
    }
    #[inline]
    pub fn update(&self, album: &SharedAlbum) {
        let ui = self.imp();
        let album_locked = album.lock().unwrap();
        let songs = &album_locked.songs;

        let mut info = songs[0].info(); // IDEA: Load artwork in the background?
        let detailed_info = info.load_detailed();
        // SAFETY: `load_detailed` ensures the value is `Some`
        let artwork = unsafe { detailed_info.as_ref().unwrap_unchecked().artwork.as_ref() };
        if artwork.is_some() {
            ui.album_cover.set_paintable(artwork);
        } else {
            ui.album_cover.set_paintable(Some(&fallback_album_image()));
        }

        self.set_title(&["Album: ", &album_locked.title].concat());
        ui.album.replace(Some(Arc::clone(album)));
        ui.album_title.set_label(&album_locked.title);
        ui.artist_name
            .set_label(&album_locked.artist.lock().unwrap().name);
        ui.year.set_label(&match album_locked.year {
            year if year > 0 => year.to_string(),
            _ => String::new(),
        });

        ui.rating.connect_rating_set(|rating| {
            println!("TODO: Decide how to handle album ratings (requested rating: {rating})");
        });

        // TODO: Divide discs into separate groups
        ui.songs_list.remove_all();
        for (i, song) in album_locked.songs.iter().enumerate() {
            let song_row = SongRow::new();

            let mut info = song.info();
            let info = info.load_basic();
            // SAFETY: `load_basic` ensures the value is `Some`
            let info = unsafe { info.as_ref().unwrap_unchecked() };
            song_row.add_prefix(
                &gtk::Label::builder()
                    .width_chars(2)
                    .label(info.track.to_string())
                    .justify(gtk::Justification::Center)
                    .css_classes(["dimmed", "numeric"])
                    .build(),
            );
            song_row.set_title(&info.title);
            let duration = song.info().load_basic().as_ref().unwrap().duration_ms;
            song_row.set_suffix_label(&format_duration_ms(duration));

            let song = Arc::clone(song);
            let album = Arc::clone(album);
            song_row.connect_activated(move |_| {
                UI_TX
                    .get()
                    .expect(EXP_INIT)
                    .send(UpdateUI::SongPage(Box::new((
                        i,
                        song.clone(),
                        Box::new(album.clone() as SharedAlbum),
                    ))))
                    .expect(EXP_RX);
            });

            ui.songs_list.append(&song_row);
        }
    }
}
