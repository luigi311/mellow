use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{Artists, ToQueue, ToShuffledQueue, search};
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::ui::artist_object::ArtistObject;
use crate::ui::item_tile::ItemTile;
use crate::ui::{UI_TX, UpdateUI};

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
    search_query: Rc<RefCell<String>>,

    artists: RefCell<Vec<ArtistObject>>,
    filter: Rc<RefCell<gtk::CustomFilter>>,
    sorter: Rc<RefCell<gtk::CustomSorter>>,
}

#[gtk::template_callbacks]
impl ArtistsPage {
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
        let model = self.artists_grid.model().expect(EXP_INIT);
        let n_items = model.n_items();
        let mut artists = Vec::with_capacity(n_items as usize);

        for i in 0..n_items {
            artists.push(
                model
                    .item(i)
                    .unwrap()
                    .downcast_ref::<ArtistObject>()
                    .unwrap()
                    .shared_artist(),
            );
        }

        let player_tx = PLAYER_TX.get().expect(EXP_INIT);
        player_tx
            .send(PlayerRequest::LoadQueue(
                match shuffle {
                    true => artists.to_shuffled_queue(),
                    false => artists.to_queue(),
                },
                None,
                0,
            ))
            .expect(EXP_RX);
        let _ = player_tx.send(PlayerRequest::TogglePlay(Some(true)));
        let ui_tx = UI_TX.get().expect(EXP_INIT);
        ui_tx.send(UpdateUI::OpenSheet(false)).expect(EXP_RX);
    }

    pub fn load_artists(&self, artsits: &Artists) {
        if artsits.is_empty() {
            self.artists_grid.set_model(None::<&gtk::NoSelection>);
            self.view_stack.set_visible_child_name("empty");
            return;
        }
        self.view_stack.set_visible_child_name("artists");

        let model = gio::ListStore::new::<ArtistObject>();
        let artists: Vec<ArtistObject> = (0..artsits.len())
            .map(|index| {
                let artist = &artsits[index];
                let artist_locked = artist.lock().unwrap();
                ArtistObject::new(
                    index as u32,
                    &artist_locked.name,
                    artist_locked.albums.len() as u64,
                    Arc::clone(artist),
                )
            })
            .collect();
        model.extend_from_slice(&artists);
        self.artists.replace(artists);

        let query = Rc::clone(&self.search_query);
        let filter = gtk::CustomFilter::new(move |object| {
            let artist_object = object.downcast_ref::<ArtistObject>().unwrap();
            let score = search::query_score(&query.borrow(), &artist_object.artist());
            artist_object.set_rank(score);
            (artist_object.artist().to_lowercase()).contains(&query.borrow().to_lowercase())
                || score >= search::SCORE_THRESHOLD
        });
        let filter_model = gtk::FilterListModel::new(Some(model), Some(filter.clone()));
        self.filter.replace(filter);

        let sorter = gtk::CustomSorter::new(|object_a, object_b| {
            let artist_a = object_a.downcast_ref::<ArtistObject>().unwrap();
            let artist_b = object_b.downcast_ref::<ArtistObject>().unwrap();
            match artist_b.rank().total_cmp(&artist_a.rank()) {
                cmp::Ordering::Equal => artist_a.artist().cmp(&artist_b.artist()),
                ordering => ordering,
            }
            .into()
        });
        let sort_model = gtk::SortListModel::new(Some(filter_model), Some(sorter.clone()));
        self.sorter.replace(sorter);

        self.artists_grid
            .set_model(Some(&gtk::NoSelection::new(Some(sort_model))));
    }

    pub fn assign_artwork(&self, index: u32, artwork: Option<gdk::Texture>) {
        self.artists.borrow()[index as usize].set_property("artwork", artwork);
    }

    pub fn uninit(&self) {
        // for artist in self.artists.take() {
        //     artist.imp().is_visible.store(false, Ordering::Relaxed);
        // }
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
        self.artists_grid.connect_activate(|grid, index| {
            let index = grid
                .model()
                .unwrap()
                .item(index)
                .unwrap()
                .downcast_ref::<ArtistObject>()
                .unwrap()
                .index();
            UI_TX
                .get()
                .expect(EXP_INIT)
                .send(UpdateUI::ArtistPageByIndex(index as usize))
                .expect(EXP_RX);
        });

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let artist_tile = ItemTile::default();
            artist_tile.set_width_request(180);
            artist_tile.set_margin_top(8);
            artist_tile.set_margin_bottom(8);
            artist_tile.imp().image.set_visible(false);
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&artist_tile));
        });
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let artist_object = list_item
                .item()
                .and_downcast::<ArtistObject>()
                .expect("Needs to be ArtistObject");
            let artist_tile = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ItemTile>()
                .expect("Needs to be ItemTile");

            artist_tile.set_info(
                &artist_object.artist(),
                &format!("Albums: {}", artist_object.albums()),
            );
            // artist_tile.set_artwork(&artist_object.artwork().unwrap_or_else(|| {
            //     artist_object.load_artwork();
            //     fallback_artist_image()
            // }));

            // artist_tile.add_bindings(&[artist_object
            //     .bind_property("artwork", &artist_tile.imp().image.get(), "paintable")
            //     .sync_create()
            //     .build()]);
        });
        factory.connect_unbind(|_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let artist_object = list_item
                .item()
                .and_downcast::<ArtistObject>()
                .expect("Needs to be AlbumObject");
            let artist_tile = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ItemTile>()
                .expect("Needs to be ItemTile");

            artist_tile.reset_bindings();
            artist_object.unload_artwork();
        });

        self.artists_grid.set_factory(Some(&factory));
    }
}
impl WidgetImpl for ArtistsPage {}
impl NavigationPageImpl for ArtistsPage {}
