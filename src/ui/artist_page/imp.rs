use adw::subclass::prelude::*;
use glib::types::StaticType;
use gtk::{CompositeTemplate, glib};
use std::cell::RefCell;
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::artist::SharedArtist;
use crate::library::{LIBRARY_TX, LibraryRequest};
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::ui::song_row::SongRow;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/artist_page.ui")]
pub struct ArtistPage {
    pub artist: RefCell<Option<SharedArtist>>,

    #[template_child]
    pub artist_name: TemplateChild<gtk::Label>,
    #[template_child]
    pub albums_list: TemplateChild<gtk::ListBox>,
}

#[gtk::template_callbacks]
impl ArtistPage {
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
            .send(LibraryRequest::PlayArtist(Arc::clone(
                self.artist.borrow().as_ref().unwrap(),
            )))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_play_shuffled(&self) {
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetShuffle(false))
            .expect(EXP_RX);
        LIBRARY_TX
            .get()
            .expect(EXP_INIT)
            .send(LibraryRequest::ShuffleArtist(Arc::clone(
                self.artist.borrow().as_ref().unwrap(),
            )))
            .expect(EXP_RX);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ArtistPage {
    const NAME: &str = "MellowArtistPage";
    type Type = super::ArtistPage;
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

impl ObjectImpl for ArtistPage {}
impl WidgetImpl for ArtistPage {}
impl NavigationPageImpl for ArtistPage {}
