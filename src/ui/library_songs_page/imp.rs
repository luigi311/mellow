use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::OnceCell;
use std::sync::mpsc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::LibraryRequest;
use crate::player::PlayerRequest;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_songs_page.ui")]
pub struct LibrarySongsPage {
    pub library_tx: OnceCell<mpsc::SyncSender<LibraryRequest>>,
    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
}

#[gtk::template_callbacks]
impl LibrarySongsPage {
    #[template_callback]
    pub fn handle_play_sequential(&self) {
        self.play_now(false);
    }

    #[template_callback]
    pub fn handle_play_shuffled(&self) {
        self.play_now(true);
    }

    fn play_now(&self, shuffle: bool) {
        let player_tx = self.player_tx.get().expect(EXP_INIT);
        let library_tx = self.library_tx.get().expect(EXP_INIT);
        player_tx
            .send(PlayerRequest::SetShuffle(shuffle))
            .expect(EXP_RX);
        library_tx
            .send(LibraryRequest::QueueAllSongs)
            .expect(EXP_RX);
        player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for LibrarySongsPage {
    const NAME: &str = "MellowLibrarySongsPage";
    type Type = super::LibrarySongsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for LibrarySongsPage {}
impl WidgetImpl for LibrarySongsPage {}
impl NavigationPageImpl for LibrarySongsPage {}
