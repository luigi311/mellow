use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;
use std::sync::MutexGuard;

use crate::library::Album;
use crate::ui::fallback_album_image;
use crate::ui::song_row::SongRow;

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

    pub fn update(&self, index: usize, album: &MutexGuard<Album>) {
        let ui = self.imp();
        let songs = &album.songs;

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
        ui.album_title.set_label(&album.title);
        ui.artist_name.set_label(&album.artist.lock().unwrap().name);
        ui.year.set_label(&match album.year {
            year if year > 0 => year.to_string(),
            _ => String::new(),
        });

        // IDEA: Divide discs into separate groups
        ui.songs_list.remove_all();
        for song in &album.songs {
            let entry = SongRow::new();
            let mut song = song.lock().unwrap();
            let mut info = song.info();
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
            // TODO: Open a song page on click
            ui.songs_list.append(&entry);
        }
    }
}
