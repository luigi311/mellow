use adw::subclass::prelude::*;
use glib::types::StaticType;
use gtk::{CompositeTemplate, glib};
use std::cell::Cell;

use crate::ui::song_row::SongRow;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/artist_page.ui")]
pub struct ArtistPage {
    pub index: Cell<usize>,
}

#[gtk::template_callbacks]
impl ArtistPage {
    #[template_callback]
    pub fn handle_play_sequential(&self) {
        println!("TODO: Play all albums by this artist (sequential)");
    }
    #[template_callback]
    pub fn handle_play_shuffled(&self) {
        println!("TODO: Play all albums by this artist (shuffled)");
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
