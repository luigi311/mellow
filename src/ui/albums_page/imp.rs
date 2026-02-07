use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::{gio, glib};
use std::cell::RefCell;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::LIBRARY_TX;
use crate::library::{Albums, LibraryRequest};
use crate::player::PLAYER_TX;
use crate::player::PlayerRequest;
use crate::ui::album_object::AlbumObject;
use crate::ui::item_tile::ItemTile;
use crate::ui::{UI_TX, UpdateUI, fallback_album_image};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/albums_page.ui")]
pub struct AlbumsPage {
    #[template_child]
    play_button: TemplateChild<adw::SplitButton>,
    #[template_child]
    shuffle_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    albums_grid: TemplateChild<gtk::GridView>,

    #[template_child]
    search_button: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    search_bar: TemplateChild<gtk::SearchBar>,
    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,
    search_query: RefCell<String>,
}

#[gtk::template_callbacks]
impl AlbumsPage {
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
        let query = self.search_query.borrow().to_string();
        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        library_tx
            .send(match shuffle {
                false => LibraryRequest::PlayAllAlbums(query),
                true => LibraryRequest::ShuffleAllAlbums(query),
            })
            .expect(EXP_RX);
    }

    pub fn load_albums(&self, albums: &Albums) {
        if albums.is_empty() {
            self.view_stack.set_visible_child_name("empty");
            return;
        }
        self.view_stack.set_visible_child_name("albums");

        let model = gio::ListStore::new::<AlbumObject>();
        // TODO: Load (or unload) `artwork` whenever an item becomes (in)visible
        let albums: Vec<AlbumObject> = (0..albums.len())
            .map(|index| {
                let album = albums[index].lock().unwrap();
                AlbumObject::new(
                    &album.title,
                    &album.artist.lock().unwrap().name,
                    album.songs[0]
                        .lock()
                        .unwrap()
                        .info()
                        .inspect_detailed()
                        .and_then(|info| info.artwork.clone()),
                )
            })
            .collect();
        model.extend_from_slice(&albums);

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&ItemTile::default()));
        });
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let object = list_item
                .item()
                .and_downcast::<AlbumObject>()
                .expect("Needs to be AlbumObject");
            let album_tile = ItemTile::builder()
                .titles(&object.album(), &object.artist())
                .artwork(&object.artwork().unwrap_or_else(fallback_album_image))
                .build();
            list_item.set_child(Some(&album_tile));
        });

        self.albums_grid
            .set_model(Some(&gtk::NoSelection::new(Some(model))));
        self.albums_grid.set_factory(Some(&factory));
    }
}

#[glib::object_subclass]
impl ObjectSubclass for AlbumsPage {
    const NAME: &str = "MellowAlbumsPage";
    type Type = super::AlbumsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}
impl ObjectImpl for AlbumsPage {
    fn constructed(&self) {
        self.albums_grid.connect_activate(|_, index| {
            UI_TX
                .get()
                .expect(EXP_INIT)
                .send(UpdateUI::AlbumPageByIndex(index as usize))
                .expect(EXP_RX);
        });
    }
}
impl WidgetImpl for AlbumsPage {}
impl NavigationPageImpl for AlbumsPage {}
