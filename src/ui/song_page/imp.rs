use adw::{prelude::*, subclass::prelude::*};
use core::cell::{Cell, RefCell};
use gtk::CompositeTemplate;
use gtk::glib;
use std::sync::Arc;

use crate::excuses::{ACTION_ERR, EXP_INIT, EXP_RX};
use crate::library::{SharedSong, SharedSongExt, ToQueue};
use crate::player::{PLAYER_TX, PlayerRequest, QueueItem};
use crate::ui::Rating;
use crate::ui::{UI_TX, UpdateUI};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/song_page.ui")]
pub struct SongPage {
    #[template_child]
    pub song_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub album_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub artist_name: TemplateChild<gtk::Label>,

    #[template_child]
    pub rating: TemplateChild<Rating>,

    pub index: Cell<usize>,
    pub shared_song: RefCell<Option<SharedSong>>,
    pub context: RefCell<Option<Box<dyn ToQueue + Send>>>,
}

#[gtk::template_callbacks]
impl SongPage {
    #[template_callback]
    pub fn handle_play_now(&self) {
        (self.obj().activate_action("ui.library_nav_pop", None)).expect(ACTION_ERR);
        let player_tx = PLAYER_TX.get().expect(EXP_INIT);
        player_tx
            .send(PlayerRequest::LoadQueue(
                self.context.borrow().as_ref().expect(EXP_INIT).to_queue(),
                None,
                self.index.get(),
            ))
            .expect(EXP_RX);
        (player_tx.send(PlayerRequest::TogglePlay(Some(true)))).expect(EXP_RX);
        let ui_tx = UI_TX.get().expect(EXP_INIT);
        ui_tx.send(UpdateUI::OpenSheet(false)).expect(EXP_RX);
        ui_tx.send(UpdateUI::FocusPlaying).expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_play_next(&self) {
        (self.obj().activate_action("ui.library_nav_pop", None)).expect(ACTION_ERR);
        let player_tx = PLAYER_TX.get().expect(EXP_INIT);
        player_tx
            .send(PlayerRequest::InsertRelative(Box::new((
                1,
                QueueItem::Song(Arc::clone(self.shared_song.borrow().as_ref().unwrap())),
            ))))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_add_to_queue(&self) {
        (self.obj().activate_action("ui.library_nav_pop", None)).expect(ACTION_ERR);
        let player_tx = PLAYER_TX.get().expect(EXP_INIT);
        player_tx
            .send(PlayerRequest::Append(QueueItem::Song(Arc::clone(
                self.shared_song.borrow().as_ref().unwrap(),
            ))))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_go_to_album(&self) {
        (UI_TX.get().expect(EXP_INIT))
            .send(UpdateUI::AlbumPage(
                self.shared_song.borrow().as_ref().unwrap().get_album(),
            ))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_go_to_artist(&self) {
        (UI_TX.get().expect(EXP_INIT))
            .send(UpdateUI::ArtistPage(
                (self.shared_song.borrow().as_ref().unwrap())
                    .album()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .artist_cloned(),
            ))
            .expect(EXP_RX);
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
