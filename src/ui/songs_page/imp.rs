use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, atomic::Ordering};

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{Songs, ToQueue, search};
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::ui::item_row::ItemRow;
use crate::ui::song_object::SongObject;
use crate::ui::{UI_TX, UpdateUI, fallback_song_image};

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
    search_query: Rc<RefCell<String>>,

    songs: RefCell<Vec<SongObject>>,
    filter: Rc<RefCell<gtk::CustomFilter>>,
    sorter: Rc<RefCell<gtk::CustomSorter>>,
}

#[gtk::template_callbacks]
impl SongsPage {
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

    #[inline]
    fn play_now(&self, shuffle: bool) {
        let model = self.songs_grid.model().expect(EXP_INIT);
        let n_items = model.n_items();
        let mut songs = Vec::with_capacity(n_items as usize);

        for i in 0..n_items {
            songs.push(
                model
                    .item(i)
                    .unwrap()
                    .downcast_ref::<SongObject>()
                    .unwrap()
                    .shared_song(),
            );
        }

        let player_tx = PLAYER_TX.get().expect(EXP_INIT);
        player_tx
            .send(PlayerRequest::LoadQueue(
                songs.to_queue(),
                match shuffle {
                    true => Some(vec![]),
                    false => None,
                },
                0,
            ))
            .expect(EXP_RX);
        let _ = player_tx.send(PlayerRequest::TogglePlay(Some(true)));
        let ui_tx = UI_TX.get().expect(EXP_INIT);
        ui_tx.send(UpdateUI::OpenSheet(false)).expect(EXP_RX);
        ui_tx.send(UpdateUI::FocusPlaying).expect(EXP_RX);
    }

    #[inline]
    pub fn load_songs(&self, songs: &Songs) {
        if songs.is_empty() {
            self.songs_grid.set_model(None::<&gtk::NoSelection>);
            self.view_stack.set_visible_child_name("empty");
            return;
        }
        self.view_stack.set_visible_child_name("songs");

        let model = gio::ListStore::new::<SongObject>();
        let songs: Vec<SongObject> = (0..songs.len())
            .map(|index| SongObject::new(index as u32, Arc::clone(&songs[index])))
            .collect();
        model.extend_from_slice(&songs);
        self.songs.replace(songs);

        let query = Rc::clone(&self.search_query);
        let filter = gtk::CustomFilter::new(move |object| {
            let song_object = object.downcast_ref::<SongObject>().unwrap();
            let score = search::query_score(
                &query.borrow().to_lowercase(),
                &song_object.song().to_lowercase(),
            );
            song_object.set_rank(score);
            score > 0.01
        });
        let filter_model = gtk::FilterListModel::new(Some(model), Some(filter.clone()));
        self.filter.replace(filter);

        let sorter = gtk::CustomSorter::new(|object_a, object_b| {
            let song_a = object_a.downcast_ref::<SongObject>().unwrap();
            let song_b = object_b.downcast_ref::<SongObject>().unwrap();
            // TODO: Should this be sorted beyond just the score?
            song_b.rank().total_cmp(&song_a.rank()).into()
        });
        let sort_model = gtk::SortListModel::new(Some(filter_model), Some(sorter.clone()));
        self.sorter.replace(sorter);

        self.songs_grid
            .set_model(Some(&gtk::NoSelection::new(Some(sort_model))));
    }

    #[inline]
    pub fn assign_artwork(&self, index: u32, artwork: Option<&gdk::Texture>) {
        self.songs.borrow()[index as usize].set_property("artwork", artwork);
    }

    pub fn uninit(&self) {
        for song in self.songs.take() {
            song.imp().is_visible.store(false, Ordering::Relaxed);
        }
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
        self.songs_grid.connect_activate(|grid, index| {
            let index = grid
                .model()
                .unwrap()
                .item(index)
                .unwrap()
                .downcast_ref::<SongObject>()
                .unwrap()
                .index();
            UI_TX
                .get()
                .expect(EXP_INIT)
                .send(UpdateUI::SongPageByIndex(index as usize))
                .expect(EXP_RX);
        });

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
            let song_object = list_item
                .item()
                .and_downcast::<SongObject>()
                .expect("Needs to be SongObject");
            let song_row = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ItemRow>()
                .expect("Needs to be ItemRow");

            song_row.set_info(&song_object.song(), &song_object.artist());
            song_row.set_artwork(&song_object.artwork().unwrap_or_else(|| {
                song_object.load_artwork();
                fallback_song_image()
            }));

            song_row.add_bindings(&[song_object
                .bind_property("artwork", &song_row.imp().image.get(), "paintable")
                .sync_create()
                .build()]);
        });
        factory.connect_unbind(|_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem");
            let song_object = list_item
                .item()
                .and_downcast::<SongObject>()
                .expect("Needs to be AlbumObject");
            let song_row = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ItemRow>()
                .expect("Needs to be ItemTile");

            song_row.reset_bindings();
            song_object.unload_artwork();
        });

        self.songs_grid.set_factory(Some(&factory));
    }
}
impl WidgetImpl for SongsPage {}
impl NavigationPageImpl for SongsPage {}
