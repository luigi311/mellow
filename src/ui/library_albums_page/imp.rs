use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::LIBRARY_TX;
use crate::library::{Albums, LibraryRequest};
use crate::player::PLAYER_TX;
use crate::player::PlayerRequest;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_albums_page.ui")]
pub struct LibraryAlbumsPage {
    #[template_child]
    play_button: TemplateChild<adw::SplitButton>,
    #[template_child]
    shuffle_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    search_bar: TemplateChild<gtk::SearchBar>,
    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,
    search_query: Rc<RefCell<String>>,
}

#[gtk::template_callbacks]
impl LibraryAlbumsPage {
    pub fn init_search(&self) {
        self.search_entry.connect_search_changed(glib::clone!(
            #[strong(rename_to=search_query)]
            self.search_query,
            move |entry| {
                search_query.replace(entry.text().to_string());
            }
        ));
    }

    #[template_callback]
    pub fn handle_toggle_search(&self, toggle: gtk::ToggleButton) {
        self.search_bar.set_search_mode(toggle.is_active());
    }

    #[template_callback]
    pub fn handle_play_sequential(&self) {
        self.play_button.set_visible(true);
        self.shuffle_button.set_visible(false);
        self.play_now(false);
    }

    #[template_callback]
    pub fn handle_play_shuffled(&self) {
        self.play_button.set_visible(false);
        self.shuffle_button.set_visible(true);
        self.play_now(true);
    }

    fn play_now(&self, shuffle: bool) {
        let player_tx = PLAYER_TX.get().expect(EXP_INIT);
        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let query = self.search_query.borrow().to_string();
        player_tx
            .send(PlayerRequest::SetShuffle(false))
            .expect(EXP_RX);
        library_tx
            .send(match shuffle {
                false => LibraryRequest::PlayAllAlbums(query),
                true => LibraryRequest::ShuffleAllAlbums(query),
            })
            .expect(EXP_RX);
    }

    pub fn load_albums(&self, albums: &Albums) {
        println!("TODO: Create a list of library albums in the UI");
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
