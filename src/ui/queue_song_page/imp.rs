use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::{Cell, OnceCell};
use std::sync::mpsc;
use tokio::sync::mpsc as tokio_mpsc;

use crate::excuses::{ACTION_ERR, EXP_INIT, EXP_RX};
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::UpdateUI;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/queue_song_page.ui")]
pub struct QueueSongPage {
    pub index: Cell<usize>,
    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
    pub ui_tx: OnceCell<tokio_mpsc::Sender<UpdateUI>>,

    #[template_child]
    pub song_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub album_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub artist_name: TemplateChild<gtk::Label>,
}

#[gtk::template_callbacks]
impl QueueSongPage {
    #[template_callback]
    pub fn handle_play_now(&self) {
        let player_tx = self.player_tx.get().expect(EXP_INIT);
        player_tx
            .send(PlayerRequest::SkipTo(self.index.get()))
            .expect(EXP_RX);
        player_tx
            .send(PlayerRequest::TogglePlay(Some(true)))
            .expect(EXP_RX);
        self.obj()
            .activate_action("ui.close_sheet", None)
            .expect(ACTION_ERR);
        self.obj()
            .activate_action("ui.playing_nav_pop", None)
            .expect(ACTION_ERR);
    }
    #[template_callback]
    pub fn handle_stop_after(&self) {
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::InsertAt(Box::new((
                self.index.get() + 1,
                QueueItem::Stopper,
            ))))
            .expect(EXP_RX);
        self.obj()
            .activate_action("ui.playing_nav_pop", None)
            .expect(ACTION_ERR);
    }
    #[template_callback]
    pub fn handle_remove_item(&self) {
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::RemoveAt(self.index.get()))
            .expect(EXP_RX);
        self.obj()
            .activate_action("ui.playing_nav_pop", None)
            .expect(ACTION_ERR);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for QueueSongPage {
    const NAME: &str = "MellowQueueSongPage";
    type Type = super::QueueSongPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for QueueSongPage {}
impl WidgetImpl for QueueSongPage {}
impl NavigationPageImpl for QueueSongPage {}
