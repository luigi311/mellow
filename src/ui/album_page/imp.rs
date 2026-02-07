use adw::subclass::prelude::*;
use glib::types::StaticType;
use gtk::{CompositeTemplate, glib};
use std::cell::RefCell;
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::album::SharedAlbum;
use crate::library::{LIBRARY_TX, LibraryRequest};
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::ui::rating::Rating;
use crate::ui::song_row::SongRow;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/album_page.ui")]
pub struct AlbumPage {
    pub album: RefCell<Option<SharedAlbum>>,

    #[template_child]
    pub album_cover: TemplateChild<gtk::Picture>,
    #[template_child]
    pub album_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub artist_name: TemplateChild<gtk::Label>,
    #[template_child]
    pub year: TemplateChild<gtk::Label>,

    #[template_child]
    pub rating: TemplateChild<Rating>,

    #[template_child]
    pub songs_list: TemplateChild<gtk::ListBox>,
}

#[gtk::template_callbacks]
impl AlbumPage {
    #[template_callback]
    pub fn handle_play_sequential(&self) {
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetShuffle(false))
            .expect(EXP_RX);
        LIBRARY_TX
            .get()
            .expect(EXP_INIT)
            .send(LibraryRequest::PlayAlbum(Arc::clone(
                self.album.borrow().as_ref().expect(EXP_INIT),
            )))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_play_shuffled(&self) {
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetShuffle(true))
            .expect(EXP_RX);
        LIBRARY_TX
            .get()
            .expect(EXP_INIT)
            .send(LibraryRequest::PlayAlbum(Arc::clone(
                self.album.borrow().as_ref().expect(EXP_INIT),
            )))
            .expect(EXP_RX);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for AlbumPage {
    const NAME: &str = "MellowAlbumPage";
    type Type = super::AlbumPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        SongRow::static_type();

        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for AlbumPage {}
impl WidgetImpl for AlbumPage {}
impl NavigationPageImpl for AlbumPage {}
