use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::glib;
use std::sync::Arc;

use crate::library::{ToQueue, song::SharedSong};

mod imp;

glib::wrapper! {
    pub struct SongPage(ObjectSubclass<imp::SongPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for SongPage {
    #[inline]
    fn default() -> Self {
        Object::builder().build()
    }
}

impl SongPage {
    #[inline]
    pub fn new(index: usize, song: SharedSong, to_queue: Box<dyn ToQueue + Send>) -> SongPage {
        let song_page = Self::default();
        song_page.update(index, song, to_queue);
        song_page
    }
    #[inline]
    pub fn update(&self, index: usize, song: SharedSong, to_queue: Box<dyn ToQueue + Send>) {
        let song_page = self.imp();

        song_page.index.set(index);
        let mut info = song.info();

        let song_info_temp = info.load_basic();
        // SAFETY: `load_basic` ensures the value is `Some`
        let song_info = unsafe { song_info_temp.as_ref().unwrap_unchecked() };
        self.set_title(&["Song: ", &song_info.title].concat());
        song_page.song_title.set_label(&song_info.title);
        song_page.album_title.set_label(&song_info.album);
        song_page.artist_name.set_label(&song_info.artist);
        song_page.context.replace(Some(to_queue));
        drop(song_info_temp);

        let user_info = info.user();
        song_page.rating.set_rating_silent(user_info.rating);
        drop(user_info);
        drop(info);

        song_page.shared_song.replace(Some(Arc::clone(&song)));
        song_page.rating.connect_rating_set(move |rating| {
            song.info().set_rating(rating);
        });
    }
}
