use adw::{prelude::*, subclass::prelude::*};
use core::cell::{Cell, OnceCell, RefCell};
use core::sync::atomic::Ordering;
use gtk::CompositeTemplate;
use gtk::{gdk, gio, glib};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{Albums, ToQueue, ToShuffledQueue};
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::ui::{AlbumObject, AlbumOrdering, ItemTile, SortConfig};
use crate::ui::{UI_TX, UpdateUI, fallback_album_image};
use crate::util::search;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/albums_page.ui")]
pub struct AlbumsPage {
    #[template_child]
    play_button: TemplateChild<adw::SplitButton>,
    #[template_child]
    sort_button: TemplateChild<adw::SplitButton>,

    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    albums_grid: TemplateChild<gtk::GridView>,

    #[template_child]
    search_entry: TemplateChild<gtk::SearchEntry>,
    search_query: Rc<RefCell<String>>,

    albums: RefCell<Vec<AlbumObject>>,
    filter: Rc<RefCell<gtk::CustomFilter>>,
    sorter: Rc<RefCell<gtk::CustomSorter>>,

    sort_mode: OnceCell<SortConfig<AlbumOrdering>>,

    shuffle: Cell<bool>,
}

#[gtk::template_callbacks]
impl AlbumsPage {
    #[inline]
    pub fn init_search(&self) {
        let filter = Rc::clone(&self.filter);
        let sorter = Rc::clone(&self.sorter);
        let search_query = Rc::clone(&self.search_query);
        self.search_entry.connect_search_changed(move |entry| {
            search_query.replace(entry.text().to_string());
            filter.borrow().changed(gtk::FilterChange::Different);
            sorter.borrow().changed(gtk::SorterChange::Different);
        });
        // TODO: Empty the query when pressing escape
        // TODO: Focus the search bar with CTRL+F
    }

    #[template_callback]
    pub fn handle_play_now(&self) {
        let model = self.albums_grid.model().expect(EXP_INIT);
        let n_items = model.n_items();
        let mut albums = Vec::with_capacity(n_items as usize);

        for i in 0..n_items {
            albums.push(
                (model.item(i).unwrap().downcast_ref::<AlbumObject>())
                    .unwrap()
                    .shared_album(),
            );
        }

        let player_tx = PLAYER_TX.get().expect(EXP_INIT);
        player_tx
            .send(PlayerRequest::LoadQueue(
                match self.shuffle.get() {
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

    #[inline]
    pub fn set_shuffle(&self, shuffle: bool) {
        self.shuffle.set(shuffle);
        self.play_button.set_icon_name(match shuffle {
            false => "media-playback-start-symbolic",
            true => "media-playlist-shuffle-symbolic",
        });
    }
    #[inline]
    #[must_use]
    pub const fn get_shuffle(&self) -> bool {
        self.shuffle.get()
    }

    #[inline]
    pub async fn load_albums(&self, albums: &Albums) {
        if albums.is_empty() {
            self.albums_grid.set_model(None::<&gtk::NoSelection>);
            self.view_stack.set_visible_child_name("empty");
            return;
        }
        self.view_stack.set_visible_child_name("albums");

        // FIX: Slight stutter caused by object constuction
        let async_timeout = Duration::from_millis(1000 / 60);
        let mut async_timer = Instant::now();
        let mut album_objects = Vec::with_capacity(albums.len());
        for index in 0..albums.len() {
            // SAFETY: The range is `0..albums.len()`
            let album = unsafe { albums.get_unchecked(index) }.lock().unwrap();
            album_objects.push(AlbumObject::new(
                index as u32,
                &album.title,
                &album.artist.lock().unwrap().name,
                album.year as u32,
                Arc::clone(&album.songs[0]),
            ));
            if async_timer.elapsed() > async_timeout {
                glib::timeout_future(Duration::from_millis(50)).await;
                async_timer = Instant::now();
            }
        }
        let model = gio::ListStore::new::<AlbumObject>();
        model.extend_from_slice(&album_objects);
        self.update_sort_fields(&model);
        self.albums.replace(album_objects);

        let query = Rc::clone(&self.search_query);
        let filter = gtk::CustomFilter::new(move |object| {
            let album_object = object.downcast_ref::<AlbumObject>().unwrap();
            let lowercase_query = &query.borrow().to_lowercase();
            let score = search::query_score(lowercase_query, &album_object.album().to_lowercase())
                .max(
                    search::query_score(lowercase_query, &album_object.artist().to_lowercase())
                        / 4.0,
                );
            album_object.set_rank(score);
            score > 0.01
        });
        let filter_model = gtk::FilterListModel::new(Some(model), Some(filter.clone()));
        self.filter.replace(filter);

        let sort_mode = *self.sort_mode.get().unwrap();
        let sorter = gtk::CustomSorter::new(move |object_a, object_b| {
            let album_a = object_a.downcast_ref::<AlbumObject>().unwrap();
            let album_b = object_b.downcast_ref::<AlbumObject>().unwrap();
            album_a.order_cmp(album_b, sort_mode)
        });
        let sort_model = gtk::SortListModel::new(Some(filter_model), Some(sorter.clone()));
        self.sorter.replace(sorter);

        self.albums_grid
            .set_model(Some(&gtk::NoSelection::new(Some(sort_model))));
    }

    #[inline]
    pub fn update_sort_fields<M>(&self, model: &M)
    where
        M: IsA<gio::ListModel> + ListModelExt,
    {
        let mut i = 0;
        while let Some(item) = model.item(i) {
            let album = item.downcast_ref::<AlbumObject>().unwrap();
            let shared_album = album.shared_album();
            let album_locked = shared_album.lock().unwrap();

            album.set_rating(album_locked.sort_rating(3.0));
            album.set_played(album_locked.average_play_count());

            // SAFETY: An album with no songs is never constructed
            let song = unsafe { album_locked.songs.get_unchecked(0) };
            let song_info = song.info();

            album.set_modified(song_info.user().modified);
            album.set_added(song_info.user().added);

            i += 1;
        }
    }

    #[inline]
    pub fn assign_artwork(&self, index: usize, artwork: Option<&gdk::Texture>) {
        let albums = self.albums.borrow();
        if index < albums.len() {
            albums[index].set_property("artwork", artwork);
        }
    }

    #[template_callback]
    pub fn handle_reverse_sort(&self) {
        let reversed = self.sort_mode.get().expect(EXP_INIT).reversed;
        let reverse = !reversed.get();
        reversed.set(reverse);
        self.sorter.borrow().changed(gtk::SorterChange::Inverted);
        self.sort_button.set_icon_name(match reverse {
            true => "view-sort-ascending-symbolic",
            false => "view-sort-descending-symbolic",
        });
    }
    #[inline]
    pub fn set_sort_mode(&self, sort_mode: AlbumOrdering) {
        let ordering = self.sort_mode.get().expect(EXP_INIT).ordering;
        ordering.replace(sort_mode);
        self.sorter.borrow().changed(gtk::SorterChange::Different);
        if let Some(model) = &self.albums_grid.model() {
            self.update_sort_fields(model);
        }
    }
    #[inline]
    #[must_use]
    pub fn get_sort_mode(&self) -> &SortConfig<AlbumOrdering> {
        self.sort_mode.get().expect(EXP_INIT)
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
        let _ = self
            .sort_mode
            .set(SortConfig::new(AlbumOrdering::Default, false));
        self.init_search();

        self.albums_grid.connect_activate(|grid, index| {
            let album = (grid.model().unwrap().item(index).unwrap())
                .downcast_ref::<AlbumObject>()
                .unwrap()
                .shared_album();
            (UI_TX.get().expect(EXP_INIT))
                .send(UpdateUI::AlbumPage(album))
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

            album_tile.add_binding(
                album_object
                    .bind_property("artwork", &album_tile.imp().image.get(), "paintable")
                    .sync_create()
                    .build(),
            );
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
