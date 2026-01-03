use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::Cell;

use crate::excuses::{ACTION_ERR, EXP_INIT, EXP_RX};
use crate::player::PLAYER_TX;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/queue_song_page.ui")]
pub struct QueueSongPage {
    pub index: Cell<usize>,
    pub stop_after: Cell<bool>,

    #[template_child]
    pub song_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub album_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub artist_name: TemplateChild<gtk::Label>,
    #[template_child]
    pub stop_after_button: TemplateChild<adw::ActionRow>,
}

#[gtk::template_callbacks]
impl QueueSongPage {
    #[template_callback]
    pub fn handle_play_now(&self) {
        let player_tx = PLAYER_TX.get().expect(EXP_INIT);
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
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send({
                let index = self.index.get() + 1;
                match self.stop_after.get() {
                    false => PlayerRequest::InsertAt(Box::new((index, QueueItem::Stopper))),
                    true => PlayerRequest::RemoveAt(index),
                }
            })
            .expect(EXP_RX);
        self.obj()
            .activate_action("ui.playing_nav_pop", None)
            .expect(ACTION_ERR);
    }
    #[template_callback]
    pub fn handle_remove_item(&self) {
        PLAYER_TX
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
