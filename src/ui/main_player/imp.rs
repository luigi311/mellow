use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::prelude::*;
use gtk::{gdk, glib};

use std::cell::OnceCell;
use std::sync::mpsc;

use crate::player::PlayerRequest;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/main_player.ui")]
pub struct MainPlayer {
    // #[template_child]
    // player_box: TemplateChild<gtk::Box>,
    #[template_child]
    song_info: TemplateChild<gtk::Box>,
    #[template_child]
    pub album_cover: TemplateChild<gtk::Picture>,
    #[template_child]
    pub song_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub album_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub artist_name: TemplateChild<gtk::Label>,

    #[template_child]
    pub media_controls: TemplateChild<gtk::Box>,
    #[template_child]
    pub pause_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub seek_bar: TemplateChild<gtk::Scale>,
    #[template_child]
    pub time_cur_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub time_end_label: TemplateChild<gtk::Label>,

    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
}

#[gtk::template_callbacks]
impl MainPlayer {
    #[template_callback]
    pub fn handle_skip_prev(&self) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::SkipPrevious)
            .unwrap();
    }
    #[template_callback]
    pub fn handle_play_pause(&self) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::TogglePlay(None))
            .unwrap();
    }
    #[template_callback]
    pub fn handle_skip_next(&self) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::SkipNext)
            .unwrap();
    }
    #[template_callback]
    pub fn handle_seek(&self, _: gtk::ScrollType, value: f64) -> glib::Propagation {
        if crate::approx_eq(value, self.seek_bar.value()) {
            return glib::Propagation::Stop;
        }
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::Seek(value))
            .unwrap();
        glib::Propagation::Proceed
    }
}

#[glib::object_subclass]
impl ObjectSubclass for MainPlayer {
    const NAME: &str = "MellowMainPlayer";
    type Type = super::MainPlayer;
    type ParentType = gtk::Box;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for MainPlayer {
    fn constructed(&self) {
        self.album_cover
            .set_paintable(Some(&gdk::Paintable::new_empty(1, 1)));
    }
}
impl WidgetImpl for MainPlayer {}
impl BoxImpl for MainPlayer {}
