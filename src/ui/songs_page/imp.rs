use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::{gio, glib};
use std::cell::RefCell;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::LIBRARY_TX;
use crate::library::{LibraryRequest, Songs};
use crate::player::PLAYER_TX;
use crate::player::PlayerRequest;
use crate::ui::item_row::ItemRow;
use crate::ui::song_object::SongObject;
use crate::ui::{UI_TX, UpdateUI};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/songs_page.ui")]
pub struct SongsPage {
    // TODO: Remember last play mode between sessions
    // (and maybe reuse the same widget?)
    #[template_child]
    play_button: TemplateChild<adw::SplitButton>,
    #[template_child]
    shuffle_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    songs_grid: TemplateChild<gtk::GridView>,

    #[template_child]
    search_button: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    search_bar: TemplateChild<gtk::SearchBar>,
    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,
    search_query: RefCell<String>,
}

#[gtk::template_callbacks]
impl SongsPage {
    pub fn init_search(&self) {
        self.search_entry.connect_search_changed(glib::clone!(
            #[strong(rename_to=search_query)]
            self.search_query,
            move |entry| {
                search_query.replace(entry.text().to_string());
            }
        ));
        self.search_button
            .bind_property("active", &self.search_bar.get(), "search-mode-enabled")
            .sync_create()
            .bidirectional()
            .build();
    }

    #[template_callback]
    pub fn handle_toggle_search(&self, toggle: &gtk::ToggleButton) {
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
        player_tx
            .send(PlayerRequest::SetShuffle(shuffle))
            .expect(EXP_RX);
        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        library_tx
            .send(LibraryRequest::PlayAllSongs(
                self.search_query.borrow().to_string(),
            ))
            .expect(EXP_RX);
    }

    pub fn load_songs(&self, songs: &Songs) {
        let model = gio::ListStore::new::<SongObject>();
        let songs: Vec<SongObject> = (0..songs.len())
            .filter_map(|index| {
                let mut song = songs[index].lock().unwrap();
                let info = song.info();
                let info = info.inspect_basic();
                info.map(|info| SongObject::new(&info.title, &info.artist))
            })
            .collect();
        model.extend_from_slice(&songs);

        if songs.is_empty() {
            self.view_stack.set_visible_child_name("empty");
            return;
        }
        self.view_stack.set_visible_child_name("songs");

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&ItemRow::default()));
        });
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let object = list_item
                .item()
                .and_downcast::<SongObject>()
                .expect("Needs to be SongObject");
            let song_tile = ItemRow::builder()
                .titles(&object.song(), &object.artist())
                .build();
            list_item.set_child(Some(&song_tile));
        });

        self.songs_grid
            .set_model(Some(&gtk::NoSelection::new(Some(model))));
        self.songs_grid.set_factory(Some(&factory));
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SongsPage {
    const NAME: &str = "MellowSongsPage";
    type Type = super::SongsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}
impl ObjectImpl for SongsPage {
    fn constructed(&self) {
        self.songs_grid.connect_activate(|_, index| {
            UI_TX
                .get()
                .unwrap()
                .send(UpdateUI::SongPageByIndex(index as usize))
                .expect(EXP_RX);
        });
    }
}
impl WidgetImpl for SongsPage {}
impl NavigationPageImpl for SongsPage {}
