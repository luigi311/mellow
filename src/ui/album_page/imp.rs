use adw::subclass::prelude::*;
use glib::types::StaticType;
use gtk::{CompositeTemplate, glib};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::album::SharedAlbum;
use crate::library::{LIBRARY_TX, LibraryRequest};
use crate::ui::rating::Rating;
use crate::ui::song_row::SongRow;
use crate::ui::{UI_TX, UpdateUI};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/album_page.ui")]
pub struct AlbumPage {
    pub album: RefCell<Option<SharedAlbum>>,

    #[template_child]
    play_button: TemplateChild<adw::SplitButton>,
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

    shuffle: Cell<bool>,
}

#[gtk::template_callbacks]
impl AlbumPage {
    #[inline]
    pub fn set_shuffle(&self, shuffle: bool) {
        self.shuffle.set(shuffle);
        self.play_button.set_icon_name(match shuffle {
            false => "media-playback-start-symbolic",
            true => "media-playlist-shuffle-symbolic",
        });
    }
    #[template_callback]
    pub fn handle_play_now(&self) {
        match self.shuffle.get() {
            true => self.play_shuffled(),
            false => self.play_sequential(),
        }
    }
    #[inline]
    pub fn play_sequential(&self) {
        (LIBRARY_TX.get().expect(EXP_INIT))
            .send(LibraryRequest::PlayAlbum(Arc::clone(
                self.album.borrow().as_ref().expect(EXP_INIT),
            )))
            .expect(EXP_RX);
    }
    #[inline]
    pub fn play_shuffled(&self) {
        (LIBRARY_TX.get().expect(EXP_INIT))
            .send(LibraryRequest::ShuffleAlbum(Arc::clone(
                self.album.borrow().as_ref().expect(EXP_INIT),
            )))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_go_to_artist(&self) {
        (UI_TX.get().expect(EXP_INIT))
            .send(UpdateUI::ArtistPage(Arc::clone(
                &self.album.borrow().as_ref().unwrap().lock().unwrap().artist,
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
