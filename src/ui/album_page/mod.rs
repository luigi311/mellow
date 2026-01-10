use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::album::AlbumMutex;
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
    fn default() -> Self {
        Object::builder().build()
    }
}

impl AlbumPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn update(&self, index: usize, album: &AlbumMutex) {
        let ui = self.imp();
        let album_locked = album.lock().unwrap();
        let songs = &album_locked.songs;

        let mut first_song = songs[0].lock().unwrap();
        let mut info = first_song.info();
        let artwork = info
            .detailed() // IDEA: Load artwork in the background?
            .artwork
            .as_ref();
        if artwork.is_some() {
            ui.album_cover.set_paintable(artwork);
        } else {
            ui.album_cover.set_paintable(Some(&fallback_album_image()));
        }
        drop(first_song);

        ui.index.set(index);
        ui.album_title.set_label(&album_locked.title);
        ui.artist_name
            .set_label(&album_locked.artist.lock().unwrap().name);
        ui.year.set_label(&match album_locked.year {
            year if year > 0 => year.to_string(),
            _ => String::new(),
        });

        // IDEA: Divide discs into separate groups
        ui.songs_list.remove_all();
        for i in 0..album_locked.songs.len() {
            // for song in &album_locked.songs {
            let entry = SongRow::new();

            let song = &album_locked.songs[i];
            let mut song_locked = song.lock().unwrap();
            let mut info = song_locked.info();
            let info = info.basic();
            entry.add_prefix(
                &gtk::Label::builder()
                    .width_chars(2)
                    .label(info.track.to_string())
                    .justify(gtk::Justification::Center)
                    .css_classes(["dimmed", "numeric"])
                    .build(),
            );
            entry.set_title(&info.title);

            entry.connect_activated({
                let song = song.clone();
                let album = album.clone();
                move |_| {
                    UI_TX
                        .get()
                        .expect(EXP_INIT)
                        .send(UpdateUI::SongPage(Box::new((
                            i,
                            song.clone(),
                            Box::new(album.clone() as AlbumMutex),
                        ))))
                        .expect(EXP_RX);
                }
            });

            ui.songs_list.append(&entry);
        }
    }
}
