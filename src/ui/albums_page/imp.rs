use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::sync::{Arc, atomic::Ordering};

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::LIBRARY_TX;
use crate::library::{Albums, LibraryRequest};
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
            self.albums_grid.set_model(None::<&gtk::NoSelection>);
            self.view_stack.set_visible_child_name("empty");
            return;
        }
        self.view_stack.set_visible_child_name("albums");

        let model = gio::ListStore::new::<AlbumObject>();
        let albums: Vec<AlbumObject> = (0..albums.len())
            .map(|index| {
                let album = albums[index].lock().unwrap();
                AlbumObject::new(
                    index as u32,
                    &album.title,
                    &album.artist.lock().unwrap().name,
                    Arc::clone(&album.songs[0]),
                )
            })
            .collect();
        model.extend_from_slice(&albums);

        self.albums_grid
            .set_model(Some(&gtk::NoSelection::new(Some(model))));
    }

    pub fn assign_artwork(&self, index: u32, artwork: Option<gdk::Texture>) {
        self.albums_grid
            .model()
            .unwrap()
            .item(index)
            .and_downcast::<AlbumObject>()
            .unwrap()
            .set_property("artwork", artwork);
    }

    pub fn uninit(&self) {
        let mut i = 0;
        while let Some(item) = self.albums_grid.model().unwrap().item(i) {
            item.downcast_ref::<AlbumObject>()
                .unwrap()
                .imp()
                .is_visible
                .store(false, Ordering::Relaxed);
            i += 1;
        }
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
            let album_object = list_item
                .item()
                .and_downcast::<AlbumObject>()
                .expect("Needs to be AlbumObject");
            let album_tile = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ItemTile>()
                .expect("Needs to be ItemTile");

            album_tile.set_info(&album_object.album(), &album_object.artist());
            // TODO: Set this on the object instead?
            album_tile.set_artwork(&album_object.artwork().unwrap_or_else(|| {
                album_object.load_artwork();
                fallback_album_image()
            }));

            album_tile.add_bindings(&[album_object
                .bind_property("artwork", &album_tile.imp().image.get(), "paintable")
                .sync_create()
                .build()]);
        });
        factory.connect_unbind(|_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let object = list_item
                .item()
                .and_downcast::<AlbumObject>()
                .expect("Needs to be AlbumObject");
            let album_tile = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ItemTile>()
                .expect("Needs to be ItemTile");

            album_tile.reset_bindings();
            object.unload_artwork();
        });

        self.albums_grid.set_factory(Some(&factory));
    }
}
impl WidgetImpl for AlbumsPage {}
impl NavigationPageImpl for AlbumsPage {}
