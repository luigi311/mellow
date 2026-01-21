use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;

use crate::library::Albums;
use crate::ui::album_row::AlbumRow;
use crate::ui::fallback_song_image;

mod imp;

glib::wrapper! {
    pub struct ArtistPage(ObjectSubclass<imp::ArtistPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for ArtistPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ArtistPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn update(&self, index: usize, artist: &str, albums: &Albums) {
        let ui = self.imp();
        ui.index.set(index);
        ui.artist_name.set_label(artist);

        ui.albums_list.remove_all();
        for album in albums {
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

            entry.connect_activated(move |_| println!("TODO: Open the album subpage"));

            ui.albums_list.append(&entry);
        }
    }
}
