use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::glib;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::LIBRARY_TX;
use crate::library::{LibraryRequest, Songs};
use crate::player::PLAYER_TX;
use crate::player::PlayerRequest;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_songs_page.ui")]
pub struct LibrarySongsPage {
    // TODO: Remember last play mode between sessions
    // (and maybe reuse the same widget?)
    #[template_child]
    play_button: TemplateChild<adw::SplitButton>,
    #[template_child]
    shuffle_button: TemplateChild<adw::SplitButton>,
}

#[gtk::template_callbacks]
impl LibrarySongsPage {
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
        player_tx
            .send(PlayerRequest::SetShuffle(shuffle))
            .expect(EXP_RX);
        library_tx.send(LibraryRequest::PlayAllSongs).expect(EXP_RX);
    }

    pub fn load_songs(&self, songs: &Songs) {
        println!("TODO: Create a list of library songs in the UI");
    }
}

#[glib::object_subclass]
impl ObjectSubclass for LibrarySongsPage {
    const NAME: &str = "MellowLibrarySongsPage";
    type Type = super::LibrarySongsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}
impl ObjectImpl for LibrarySongsPage {}
impl WidgetImpl for LibrarySongsPage {}
impl NavigationPageImpl for LibrarySongsPage {}
