use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::OnceCell;
use std::sync::mpsc;

use crate::library::LibraryRequest;
use crate::player::PlayerRequest;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_artists_page.ui")]
pub struct LibraryArtistsPage {
    pub library_tx: OnceCell<mpsc::SyncSender<LibraryRequest>>,
    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
}

#[gtk::template_callbacks]
impl LibraryArtistsPage {
    #[template_callback]
    pub fn handle_play_sequential(&self) {
        println!("TODO: Play all albums/songs in sequence");
    }

    #[template_callback]
    pub fn handle_play_shuffled(&self) {
        println!("TODO: Create a queue with randomly ordered artists but sequential albums/songs");
    }
}

#[glib::object_subclass]
impl ObjectSubclass for LibraryArtistsPage {
    const NAME: &str = "MellowLibraryArtistsPage";
    type Type = super::LibraryArtistsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for LibraryArtistsPage {}
impl WidgetImpl for LibraryArtistsPage {}
impl NavigationPageImpl for LibraryArtistsPage {}
