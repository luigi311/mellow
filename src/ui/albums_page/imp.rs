use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;
use std::sync::{Arc, atomic::Ordering};

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{Albums, ToQueue, ToShuffledQueue, search};
use crate::player::{PLAYER_TX, PlayerRequest};
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
    search_query: Rc<RefCell<String>>,

    albums: RefCell<Vec<AlbumObject>>,
    filter: Rc<RefCell<gtk::CustomFilter>>,
    sorter: Rc<RefCell<gtk::CustomSorter>>,
}

#[gtk::template_callbacks]
impl AlbumsPage {
    #[inline]
    pub fn init_search(&self) {
        let filter = Rc::clone(&self.filter);
        let sorter = Rc::clone(&self.sorter);
        self.search_entry.connect_search_changed(glib::clone!(
            #[strong(rename_to=search_query)]
            self.search_query,
            move |entry| {
                search_query.replace(entry.text().to_string());
                filter.borrow().changed(gtk::FilterChange::Different);
                sorter.borrow().changed(gtk::SorterChange::Different);
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
        let model = self.albums_grid.model().expect(EXP_INIT);
        let n_items = model.n_items();
        let mut albums = Vec::with_capacity(n_items as usize);

        for i in 0..n_items {
            albums.push(
                model
                    .item(i)
                    .unwrap()
                    .downcast_ref::<AlbumObject>()
                    .unwrap()
                    .shared_album(),
            );
        }

        let player_tx = PLAYER_TX.get().expect(EXP_INIT);
        player_tx
            .send(PlayerRequest::LoadQueue(
                match shuffle {
                    true => albums.to_shuffled_queue(),
                    false => albums.to_queue(),
                },
                None,
                0,
            ))
            .expect(EXP_RX);
        let _ = player_tx.send(PlayerRequest::TogglePlay(Some(true)));
        let ui_tx = UI_TX.get().expect(EXP_INIT);
        ui_tx.send(UpdateUI::OpenSheet(false)).expect(EXP_RX);
        ui_tx.send(UpdateUI::FocusPlaying).expect(EXP_RX);
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
                    album.year as u32,
                    Arc::clone(&album.songs[0]),
                )
            })
            .collect();
        model.extend_from_slice(&albums);
        self.albums.replace(albums);

        let query = Rc::clone(&self.search_query);
        let filter = gtk::CustomFilter::new(move |object| {
            let album_object = object.downcast_ref::<AlbumObject>().unwrap();
            let score = search::query_score(
                &query.borrow().to_lowercase(),
                &album_object.album().to_lowercase(),
            );
            album_object.set_rank(score);
            score > 0.01
        });
        let filter_model = gtk::FilterListModel::new(Some(model), Some(filter.clone()));
        self.filter.replace(filter);

        let sorter = gtk::CustomSorter::new(|object_a, object_b| {
            let album_a = object_a.downcast_ref::<AlbumObject>().unwrap();
            let album_b = object_b.downcast_ref::<AlbumObject>().unwrap();
            match album_b.rank().total_cmp(&album_a.rank()) {
                cmp::Ordering::Equal => match album_a.artist().cmp(&album_b.artist()) {
                    cmp::Ordering::Equal => match album_a.year().cmp(&album_b.year()) {
                        cmp::Ordering::Equal => album_a.album().cmp(&album_b.album()),
                        ordering => ordering,
                    },
                    ordering => ordering,
                },
                ordering => ordering,
            }
            .into()
        });
        let sort_model = gtk::SortListModel::new(Some(filter_model), Some(sorter.clone()));
        self.sorter.replace(sorter);

        self.albums_grid
            .set_model(Some(&gtk::NoSelection::new(Some(sort_model))));
    }

    #[inline]
    pub fn assign_artwork(&self, index: u32, artwork: Option<&gdk::Texture>) {
        self.albums.borrow()[index as usize].set_property("artwork", artwork);
    }

    pub fn uninit(&self) {
        for album in self.albums.take() {
            album.imp().is_visible.store(false, Ordering::Relaxed);
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
        self.albums_grid.connect_activate(|grid, index| {
            let index = grid
                .model()
                .unwrap()
                .item(index)
                .unwrap()
                .downcast_ref::<AlbumObject>()
                .unwrap()
                .index();
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
