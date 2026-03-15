use adw::{prelude::*, subclass::prelude::*};
use glib::{Object, clone};
use gtk::{Orientation, gdk, glib};
use std::sync::{Arc, atomic::Ordering};

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, Library, SharedAlbum};
use crate::ui::ListRow;
use crate::ui::{UI_TX, UpdateUI, fallback_album_image};
use crate::util::{format_duration_minutes, format_duration_ms};

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
    /// Creates a new `AlbumPage` instance using the information from `album`
    ///
    /// # Panics
    /// The function panics if any of the `album`'s `Mutex` or the `album.songs`'
    /// `RwLock`s are in a poisoned state. It may also panic at runtime upon
    /// interaction if `UI_TX` is uninitialized, or the channel is closed.
    #[inline]
    #[must_use]
    pub fn new(album: &SharedAlbum, page_index: usize) -> AlbumPage {
        let album_page = Self::default();

        let ui = album_page.imp();
        let album_locked = album.lock().unwrap();
        let songs = album_locked.songs();

        album_page.set_title(&["Album: ", album_locked.title()].concat());
        ui.album.replace(Some(Arc::clone(album)));
        ui.album_title.set_label(album_locked.title());
        ui.artist_name
            .set_label(album_locked.artist().lock().unwrap().name());
        match album_locked.year() {
            year if year > 0 => ui.year.set_label(&year.to_string()),
            _ => ui.year.set_visible(false),
        }

        ui.rating
            .set_rating_silent(album_locked.average_rating(0.0).round() as u8);
        ui.rating.connect_rating_set({
            let album = Arc::clone(album);
            move |rating| album.lock().unwrap().rate_all_songs(rating)
        });

        let mut disc_number = !0;
        let mut duration_total_ms = 0;
        let mut album_group_index = 1;
        let mut album_group = adw::PreferencesGroup::new();

        for (i, song) in album_locked.songs().iter().enumerate() {
            let song_row = ListRow::new();

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
            let duration = info.duration_ms;
            song_row.set_suffix_label(&format_duration_ms(duration));
            duration_total_ms += duration;

            let song = Arc::clone(song);
            let album = Arc::clone(album);
            song_row.connect_activated(move |_| {
                (UI_TX.get().expect(EXP_INIT))
                    .send(UpdateUI::SongPage(Box::new((
                        i,
                        Arc::clone(&song),
                        Box::new(album.clone() as SharedAlbum),
                    ))))
                    .expect(EXP_RX);
            });

            ui.details
                .set_label(&format_duration_minutes(duration_total_ms / (1000 * 60)));

            if info.disc != disc_number {
                disc_number = info.disc;
                let play_buttons = gtk::Box::new(Orientation::Horizontal, 16);
                let queue_disc_button = gtk::Button::builder()
                    // TODO: Support translations
                    .tooltip_text(format!("Add Disc {disc_number} To Queue"))
                    .icon_name("list-add-symbolic")
                    .css_name("flat")
                    .build();
                queue_disc_button.connect_clicked(clone!(
                    #[weak(rename_to=album_page)]
                    ui,
                    move |_| album_page.add_disc_to_queue(disc_number)
                ));
                queue_disc_button.set_cursor_from_name(Some("pointer"));
                let play_disc_button = gtk::Button::builder()
                    // TODO: Support translations
                    .tooltip_text(format!("Play Disc {disc_number}"))
                    .icon_name("media-playback-start-symbolic")
                    .css_name("flat")
                    .build();
                play_disc_button.connect_clicked(clone!(
                    #[weak(rename_to=album_page)]
                    ui,
                    move |_| album_page.play_disc(disc_number)
                ));
                play_disc_button.set_cursor_from_name(Some("pointer"));
                play_buttons.append(&queue_disc_button);
                play_buttons.append(&play_disc_button);
                album_group = adw::PreferencesGroup::builder()
                    // TODO: Support translations
                    .title(format!("Disc {disc_number}"))
                    .header_suffix(&play_buttons)
                    .build();
                ui.album_pref_page.insert(&album_group, album_group_index);
                album_group_index += 1;
            }

            album_group.add(&song_row);
        }

        let mut info = songs[0].info();
        let Some(ref detailed_info) = *info.inspect_detailed() else {
            match info.load_thumbnail().as_ref() {
                None => ui.album_cover.set_paintable(Some(&fallback_album_image())),
                thumbnail => ui.album_cover.set_paintable(thumbnail),
            }

            let song = Arc::clone(&songs[0]);
            let cancel = Arc::clone(&ui.cancel_artowrk_loading);
            Library::run_task(LIBRARY_TX.get().expect(EXP_RX), move || {
                if cancel.load(Ordering::Relaxed) {
                    #[cfg(debug_assertions)]
                    println!("Arwork loading cancelled");
                    return;
                }
                drop(song.info().load_detailed());
                if cancel.load(Ordering::Relaxed) {
                    #[cfg(debug_assertions)]
                    println!("Arwork assignment cancelled");
                    return;
                }
                let _ = (UI_TX.get().unwrap()).send(UpdateUI::AlbumPageLoaded(page_index, song));
            });

            return album_page;
        };

        match detailed_info.artwork.as_ref() {
            None => ui.album_cover.set_paintable(Some(&fallback_album_image())),
            artwork => ui.album_cover.set_paintable(artwork),
        }

        album_page
    }

    #[inline]
    pub fn assign_artwork(&self, artwork: Option<&gdk::Texture>) {
        if artwork.is_some() {
            self.imp().album_cover.set_paintable(artwork);
        } else {
            self.imp()
                .album_cover
                .set_paintable(Some(&fallback_album_image()));
        }
    }

    /// Sets the shuffle mode for the play button
    #[inline]
    pub fn set_shuffle(&self, shuffle: bool) {
        self.imp().set_shuffle(shuffle);
    }
}
