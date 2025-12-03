use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::OnceCell;
use std::sync::mpsc;

use crate::library::LibraryRequest;
use crate::player::PlayerRequest;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_albums_page.ui")]
pub struct LibraryAlbumsPage {
    pub library_tx: OnceCell<mpsc::SyncSender<LibraryRequest>>,
    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
    pub view_stack: OnceCell<adw::ViewStack>,
    pub sheet: OnceCell<adw::BottomSheet>,
}

#[gtk::template_callbacks]
impl LibraryAlbumsPage {
    #[template_callback]
    pub fn handle_play_sequential(&self) {
        println!("TODO: Play all artists/albums/songs in sequence");
    }

    #[template_callback]
    pub fn handle_play_shuffled(&self) {
        println!("TODO: Create a queue with randomly ordered albums but sequential songs");
    }
}

#[glib::object_subclass]
impl ObjectSubclass for LibraryAlbumsPage {
    const NAME: &str = "MellowLibraryAlbumsPage";
    type Type = super::LibraryAlbumsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for LibraryAlbumsPage {}
impl WidgetImpl for LibraryAlbumsPage {}
impl NavigationPageImpl for LibraryAlbumsPage {}
