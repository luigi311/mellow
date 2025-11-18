use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::{Cell, OnceCell};
use std::sync::mpsc;

use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/song_page.ui")]
pub struct SongPage {
    pub index: Cell<usize>,
    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
    pub navigation_view: OnceCell<adw::NavigationView>,
    pub bottom_sheet: OnceCell<adw::BottomSheet>,

    #[template_child]
    pub song_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub album_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub artist_name: TemplateChild<gtk::Label>,
    #[template_child]
    pub play_now: TemplateChild<adw::ActionRow>,
    #[template_child]
    pub stop_after: TemplateChild<adw::ActionRow>,
}

#[gtk::template_callbacks]
impl SongPage {
    #[template_callback]
    pub fn handle_play_now(&self) {
        let player_tx = self.player_tx.get().unwrap();
        player_tx
            .send(PlayerRequest::SkipTo(self.index.get()))
            .unwrap();
        player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .unwrap();
        self.navigation_view.get().unwrap().pop();
        self.bottom_sheet.get().unwrap().set_open(false);
    }
    #[template_callback]
    pub fn handle_stop_after(&self) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::InsertAt(Box::new((
                self.index.get() + 1,
                QueueItem::Stopper,
            ))))
            .unwrap();
        self.navigation_view.get().unwrap().pop();
    }
    #[template_callback]
    pub fn handle_remove_item(&self) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::RemoveAt(self.index.get()))
            .unwrap();
        self.navigation_view.get().unwrap().pop();
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SongPage {
    const NAME: &str = "MellowSongPage";
    type Type = super::SongPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for SongPage {}
impl WidgetImpl for SongPage {}
impl NavigationPageImpl for SongPage {}
