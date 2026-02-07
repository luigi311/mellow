use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

use crate::library::song::SharedSong;

mod imp;

glib::wrapper! {
    pub struct QueueSubpage(ObjectSubclass<imp::QueueSubpage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl QueueSubpage {
    pub fn update(&self, index: usize, song: SharedSong) {
        let song_page = self.imp();
        song_page.index.set(index);
        let mut song_locked = song.lock().unwrap();
        let mut info = song_locked.info();
        let song_info = info.basic();
        song_page.song_title.set_label(&song_info.title);
        song_page.album_title.set_label(&song_info.album);
        song_page.artist_name.set_label(&song_info.artist);
        let user_info = info.user();
        song_page.rating.set_rating_silent(user_info.rating);
        drop(song_locked);
        song_page.rating.connect_rating_set(move |rating| {
            song.lock().unwrap().info().set_rating(rating);
        });
    }

    pub fn set_stop_after(&self, stop_after: bool) {
        let song_page = self.imp();
        song_page.stop_after.set(stop_after);
        song_page.stop_after_button.set_title(match stop_after {
            // TODO: Support translations
            true => "Do Not Pause After",
            false => "Pause After",
        });
    }
}
