use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::{gdk, gio, glib};
use std::cell::{Cell, OnceCell, RefCell};
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, Library};
use crate::player::queue_item::QueueItem;
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::ui::queue_item_object::QueueItemObject;
use crate::ui::queue_subpage::QueueSubpage;
use crate::ui::song_row::SongRow;
use crate::ui::{UI_TX, UpdateUI, fallback_song_image};

const NUM_ITEMS_AHEAD: usize = 45;
const NUM_ITEMS_BEHIND: usize = 45;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/queue_page.ui")]
pub struct QueuePage {
    #[template_child]
    pub shuffle_toggle: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pub repeat_toggle: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    list_box: TemplateChild<gtk::ListBox>,
    // list_view: TemplateChild<gtk::ListView>,
    #[template_child]
    scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,

    playing_index: Cell<usize>,
    queue_items: RefCell<Vec<QueueItemObject>>,
    pub song_page: OnceCell<QueueSubpage>,
    list_model: OnceCell<gio::ListStore>,
}

#[gtk::template_callbacks]
impl QueuePage {
    #[template_callback]
    pub fn handle_set_repeat(&self, toggle_button: &gtk::ToggleButton) {
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetRepeat(toggle_button.is_active()))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_set_shuffle(&self, toggle_button: &gtk::ToggleButton) {
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetShuffle(toggle_button.is_active()))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_open_library(&self) {
        UI_TX
            .get()
            .expect(EXP_INIT)
            .send(UpdateUI::FocusLibrary)
            .expect(EXP_RX);
    }

    pub fn update_song_queue(&self, queue: &[QueueItem], index: usize) {
        self.view_stack
            .set_visible_child_name(match queue.is_empty() {
                true => "queue_empty",
                false => "song_queue",
            });

        // TODO: Reorder queue items using drag & drop
        // FIX: The scroll position resets when the queue is updated

        self.playing_index.set(index);
        let Some(list_model) = self.list_model.get() else {
            return;
        };

        let (start, end) = (
            index.saturating_sub(NUM_ITEMS_BEHIND),
            (index + NUM_ITEMS_AHEAD).min(queue.len()),
        );
        let items: Vec<QueueItemObject> = queue
            .iter()
            .enumerate()
            .take(end)
            .skip(start)
            .map(|index_item| {
                let object_index = index_item.0 as u32;
                match index_item.1 {
                    QueueItem::Song(song) => {
                        let mut info = song.info();

                        let queue_item_object = {
                            let song_info_temp = info.load_basic();
                            // SAFETY: `load_basic` is always safe to unwrap
                            let song_info = unsafe { song_info_temp.as_ref().unwrap_unchecked() };

                            QueueItemObject::new(
                                object_index,
                                object_index == index as u32,
                                song_info.title.clone(),
                                song_info.artist.clone(),
                                Some(Arc::clone(song)),
                            )
                        };

                        // TODO: Cached low-res album covers
                        if let Some(artwork) = info
                            .inspect_detailed()
                            .as_ref()
                            .map_or_else(|| None, |info| info.artwork.as_ref())
                        {
                            queue_item_object.set_artwork(artwork);
                        }

                        queue_item_object
                    }
                    QueueItem::Stopper => QueueItemObject::new(
                        object_index,
                        false,
                        String::from("Pause"),
                        String::new(),
                        None,
                    ),
                }
            })
            .collect();
        list_model.splice(0, list_model.n_items(), &items);
        self.queue_items.replace(items);

        let scroll_target = (index - start) * 54;
        self.scrolled_window
            .vadjustment()
            .set_value(scroll_target as f64);

        let songs = queue.to_vec();
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), move || {
            // Garbage collection
            for (index, song) in songs.iter().enumerate() {
                if !(start..end).contains(&index)
                    && let QueueItem::Song(song) = song
                    && song.info().inspect_detailed().as_ref().is_some_and(|info| {
                        info.artwork
                            .as_ref()
                            .is_some_and(|artwork| artwork.ref_count() == 1)
                    })
                {
                    song.info().unload_detailed();
                }
            }
        });
    }

    pub fn assign_artwork(&self, index: u32, artwork: Option<&gdk::Texture>) {
        let queue_items_len = self.queue_items.borrow().len();
        let playing_index = self.playing_index.get();
        if index < self.playing_index.get().saturating_sub(NUM_ITEMS_BEHIND) as u32 {
            return;
        }
        let index = index as usize + NUM_ITEMS_BEHIND.min(playing_index) - playing_index;
        if index >= queue_items_len {
            dbg!(index);
            return;
        }
        self.queue_items.borrow()[index].set_property("artwork", artwork);
    }

    pub fn uninit(&self) {
        self.list_model.get().expect(EXP_INIT).remove_all();
    }
}

#[glib::object_subclass]
impl ObjectSubclass for QueuePage {
    const NAME: &str = "MellowQueuePage";
    type Type = super::QueuePage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}
impl ObjectImpl for QueuePage {
    fn constructed(&self) {
        self.obj().update_shuffle(false);
        self.obj().update_repeat(false);

        let model = gio::ListStore::new::<QueueItemObject>();
        self.list_box.bind_model(Some(&model), move |object| {
            let queue_item_object = object.downcast_ref::<QueueItemObject>().unwrap();

            let item_row = SongRow::default();
            item_row.set_title(&queue_item_object.title());
            item_row.set_subtitle(&queue_item_object.subtitle());

            match queue_item_object.shared_song() {
                // Song
                Some(_) => {
                    if queue_item_object.playing() {
                        item_row.add_css_class("heading");
                        item_row.add_css_class("card");
                    }

                    item_row.add_bindings(&[queue_item_object
                        .bind_property("artwork", &item_row.imp().prefix_image.get(), "paintable")
                        .sync_create()
                        .build()]);

                    let artwork = queue_item_object.artwork();
                    if artwork.is_some() {
                        item_row.set_prefix_image(artwork.as_ref());
                    } else {
                        queue_item_object.load_artwork();
                        item_row.set_prefix_image(Some(&fallback_song_image()));
                    }

                    let object_index = queue_item_object.index() as usize;
                    item_row.connect_activated(move |_| {
                        UI_TX
                            .get()
                            .expect(EXP_INIT)
                            .send(UpdateUI::QueueSupbage(object_index))
                            .expect(EXP_RX);
                    });
                }
                // Stopper
                None => {
                    item_row.add_css_class("heading");
                    item_row.add_css_class("dimmed");
                    // IDEA: Draw a pause icon in place of the album cover
                }
            }

            item_row.upcast::<gtk::Widget>()
        });
        let _ = self.list_model.set(model);
    }
}

impl WidgetImpl for QueuePage {}
impl NavigationPageImpl for QueuePage {}
