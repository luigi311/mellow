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
    pub view_stack: OnceCell<adw::ViewStack>,
    pub sheet: OnceCell<adw::BottomSheet>,
}

#[gtk::template_callbacks]
impl LibrarySongsPage {
    #[template_callback]
    pub fn handle_play_sequential(&self) {
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetShuffle(false))
            .expect(EXP_RX);
        self.library_tx
            .get()
            .expect(EXP_INIT)
            .send(LibraryRequest::QueueAllSongs)
            .expect(EXP_RX);
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.view_stack
            .get()
            .expect(EXP_INIT)
            .set_visible_child_name("playing");
        self.sheet.get().expect(EXP_INIT).set_open(false);
    }

    #[template_callback]
    pub fn handle_play_shuffled(&self) {
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetShuffle(true))
            .expect(EXP_RX);
        self.library_tx
            .get()
            .expect(EXP_INIT)
            .send(LibraryRequest::QueueAllSongs)
            .expect(EXP_RX);
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.view_stack
            .get()
            .expect(EXP_INIT)
            .set_visible_child_name("playing");
        self.sheet.get().expect(EXP_INIT).set_open(false);
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
