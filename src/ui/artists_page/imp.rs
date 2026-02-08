use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::{gio, glib};
use std::cell::RefCell;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::LIBRARY_TX;
use crate::library::{Artists, LibraryRequest};
use crate::player::PLAYER_TX;
use crate::player::PlayerRequest;
use crate::ui::artist_object::ArtistObject;
use crate::ui::item_tile::ItemTile;
use crate::ui::{UI_TX, UpdateUI, fallback_artist_image};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/artists_page.ui")]
pub struct ArtistsPage {
    #[template_child]
    play_button: TemplateChild<adw::SplitButton>,
    #[template_child]
    shuffle_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    artists_grid: TemplateChild<gtk::GridView>,

    #[template_child]
    search_button: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    search_bar: TemplateChild<gtk::SearchBar>,
    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,
    search_query: RefCell<String>,
}

#[gtk::template_callbacks]
impl ArtistsPage {
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
            .send(PlayerRequest::SetShuffle(false))
            .expect(EXP_RX);
        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let query = self.search_query.borrow().to_string();
        library_tx
            .send(match shuffle {
                false => LibraryRequest::PlayAllArtists(query),
                true => LibraryRequest::ShuffleAllArtists(query),
            })
            .expect(EXP_RX);
    }

    pub fn load_artists(&self, artsits: &Artists) {
        if artsits.is_empty() {
            self.view_stack.set_visible_child_name("empty");
            return;
        }
        self.view_stack.set_visible_child_name("artists");

        // TODO: Most of this does not need to happen every time

        let model = gio::ListStore::new::<ArtistObject>();
        let albums: Vec<ArtistObject> = (0..artsits.len())
            .map(|index| {
                let artist = artsits[index].lock().unwrap();
                ArtistObject::new(&artist.name, artist.albums.len() as u64)
            })
            .collect();
        model.extend_from_slice(&albums);

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(
                    &ItemTile::builder().image_css_classes(&["circular"]).build(),
                ));
        });
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let object = list_item
                .item()
                .and_downcast::<ArtistObject>()
                .expect("Needs to be ArtistObject");
            let artist_tile = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ItemTile>()
                .expect("Needs to be ItemTile");
            artist_tile.set_info(&object.artist(), &format!("Albums: {}", object.albums()));
            artist_tile.set_artwork(&object.artwork().unwrap_or_else(|| {
                // TODO: Load artwork in the background and send a signal to assign the artwork
                fallback_artist_image()
            }));
        });
        factory.connect_unbind(|_, list_item| {
            // TODO: Unload artwork and unassign it from the object
        });

        self.artists_grid
            .set_model(Some(&gtk::NoSelection::new(Some(model))));
        self.artists_grid.set_factory(Some(&factory));
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ArtistsPage {
    const NAME: &str = "MellowArtistsPage";
    type Type = super::ArtistsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}
impl ObjectImpl for ArtistsPage {
    fn constructed(&self) {
        self.artists_grid.connect_activate(|_, index| {
            UI_TX
                .get()
                .expect(EXP_INIT)
                .send(UpdateUI::ArtistPageByIndex(index as usize))
                .expect(EXP_RX);
        });
    }
}
impl WidgetImpl for ArtistsPage {}
impl NavigationPageImpl for ArtistsPage {}
