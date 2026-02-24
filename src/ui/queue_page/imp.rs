use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::{gdk, gio, glib};
use std::cell::{Cell, OnceCell, RefCell};
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::format_duration_ms;
use crate::library::song::SharedSong;
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
    #[template_child]
    scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,

    playing_index: Cell<usize>,
    queue_item_objects: RefCell<Vec<QueueItemObject>>,
    pub song_page: OnceCell<QueueSubpage>,
    list_model: OnceCell<gio::ListStore>,
}

#[derive(Debug)]
struct IndexNotFoundError;

#[gtk::template_callbacks]
impl QueuePage {
    #[template_callback]
    pub fn handle_set_repeat(&self, toggle_button: &gtk::ToggleButton) {
        (PLAYER_TX.get().expect(EXP_INIT))
            .send(PlayerRequest::SetRepeat(toggle_button.is_active()))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_set_shuffle(&self, toggle_button: &gtk::ToggleButton) {
        (PLAYER_TX.get().expect(EXP_INIT))
            .send(PlayerRequest::SetShuffle(toggle_button.is_active()))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_open_library(&self) {
        (UI_TX.get().expect(EXP_INIT))
            .send(UpdateUI::FocusLibrary)
            .expect(EXP_RX);
    }

    #[inline]
    pub fn scroll_to(&self, scroll_target: f64) {
        let scrolled_window = self.scrolled_window.get();
        scrolled_window.vadjustment().set_value(scroll_target);
        // WORKAROUND: Setting the scroll position in an idle task because it
        // doesn't update otherwise
        glib::idle_add_local(move || {
            scrolled_window.vadjustment().set_value(scroll_target);
            glib::ControlFlow::Break
        });
    }

    pub fn scroll_to_playing(&self) {
        let index = self.playing_index.get();
        self.scroll_to(((index - index.saturating_sub(NUM_ITEMS_BEHIND)) * 54) as f64);
    }

    pub fn update_song_queue(&self, queue: &[QueueItem], index: usize) {
        self.view_stack
            .set_visible_child_name(match queue.is_empty() {
                true => "queue_empty",
                false => "song_queue",
            });

        // TODO: Reorder queue items using drag & drop

        self.playing_index.set(index);
        let Some(list_model) = self.list_model.get() else {
            return;
        };

        let start = index.saturating_sub(NUM_ITEMS_BEHIND);
        let end = (index + NUM_ITEMS_AHEAD).min(queue.len());

        // Garbage collection
        Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), {
            let queue = queue.to_vec();
            move || {
                // NOTE: Garbage collection moved to before assigning the items,
                // because it would othewise sometimes unload background-loaded
                // artworks before they would be assigned. If there are issues
                // with queue artworks not loading in the future, try disabling
                // garbage collection to verify that it is working properly.
                for (index, song) in queue.iter().enumerate() {
                    if !(start..end).contains(&index)
                        && let QueueItem::Song(song) = song
                        && (song.info().try_inspect_detailed().as_ref()).is_ok_and(|info| {
                            info.as_ref().is_some_and(|info| {
                                info.artwork
                                    .as_ref()
                                    .is_some_and(|artwork| artwork.ref_count() < 2)
                            })
                        })
                    {
                        song.info().unload_detailed();
                    }
                }
            }
        });

        let previous_scroll_position = self.scrolled_window.vadjustment().value();
        let items: Vec<QueueItemObject> = (queue.iter().enumerate().take(end).skip(start))
            .map(|index_item| {
                let q_index = index_item.0;
                match index_item.1 {
                    QueueItem::Song(song) => self.new_song(q_index as u32, q_index == index, song),
                    QueueItem::Stopper => self.new_stopper(q_index as u32),
                }
            })
            .collect();
        list_model.splice(0, list_model.n_items(), &items);
        self.queue_item_objects.replace(items);

        // Re-applying the scroll position, because it resets when the `list_box` rows change
        self.scroll_to(previous_scroll_position);

        // IDEA: Maybe scroll to the current song when the queue starts,
        // or the shuffle mode is updated?
    }

    #[inline]
    fn new_song(&self, queue_index: u32, is_playing: bool, song: &SharedSong) -> QueueItemObject {
        let object = QueueItemObject::new(queue_index, is_playing, Some(Arc::clone(song)));

        let mut info = song.info();

        let song_info_temp = info.load_basic();
        // SAFETY: `load_basic` ensures the value is `Some`
        let song_info = unsafe { song_info_temp.as_ref().unwrap_unchecked() };
        object.set_title(song_info.title.clone());
        object.set_subtitle(song_info.artist.clone());
        object.set_suffix(format_duration_ms(song_info.duration_ms));
        drop(song_info_temp);

        // TODO: Cached low-res album covers
        if let Ok(info) = info.try_inspect_detailed()
            && let Some(artwork) = info
                .as_ref()
                .map_or_else(|| None, |info| info.artwork.as_ref())
        {
            object.set_artwork(artwork);
        }

        object
    }

    #[inline]
    fn new_stopper(&self, queue_index: u32) -> QueueItemObject {
        let queue_item_object = QueueItemObject::new(queue_index, false, None);
        queue_item_object.set_title("Pause");
        queue_item_object
    }

    #[inline]
    pub fn assign_artwork(&self, index: usize, artwork: Option<&gdk::Texture>) {
        if let Ok(index) = self.queue_index_to_model(index) {
            self.queue_item_objects.borrow()[index].set_property("artwork", artwork);
        }
    }

    /// Takes a queue item index and returns the index to access its object
    ///
    /// # Panics
    /// Panics if `self.queue_item_objects` `RefCell` is mutably borrowed
    #[inline]
    fn queue_index_to_model(&self, index: usize) -> Result<usize, IndexNotFoundError> {
        let queue_items_len = self.queue_item_objects.borrow().len();
        let playing_index = self.playing_index.get();
        if index < playing_index.saturating_sub(NUM_ITEMS_BEHIND) {
            return Err(IndexNotFoundError);
        }
        let model_index = index + NUM_ITEMS_BEHIND.min(playing_index) - playing_index;
        if model_index >= queue_items_len {
            return Err(IndexNotFoundError);
        }
        Ok(model_index)
    }
    // /// Takes a model index and returns the index to access its queue item
    // #[inline]
    // #[must_use]
    // fn model_index_to_queue(&self, index: usize) -> usize {
    //     let playing_index = self.playing_index.get();
    //     index + playing_index - NUM_ITEMS_BEHIND.min(playing_index)
    // }

    /// Empties the list model, cancelling any pending background tasks during drop
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

            let queue_row = SongRow::default();
            queue_row.set_title(&queue_item_object.title());
            queue_row.set_subtitle(&queue_item_object.subtitle());

            if queue_item_object.shared_song().is_some() {
                // The queue item is a `Song`

                if queue_item_object.playing() {
                    queue_row.add_css_class("heading");
                    queue_row.add_css_class("card");
                }

                queue_row.add_bindings(&[queue_item_object
                    .bind_property("artwork", &queue_row.imp().prefix_image.get(), "paintable")
                    .sync_create()
                    .build()]);

                queue_row.set_suffix_label(&queue_item_object.suffix());

                let artwork = queue_item_object.artwork();
                if artwork.is_some() {
                    queue_row.set_prefix_image(artwork.as_ref());
                } else {
                    queue_item_object.load_artwork();
                    queue_row.set_prefix_image(Some(&fallback_song_image()));
                }

                let object_index = queue_item_object.index() as usize;
                queue_row.connect_activated(move |_| {
                    (UI_TX.get().expect(EXP_INIT))
                        .send(UpdateUI::QueueSupbage(object_index))
                        .expect(EXP_RX);
                });
            } else {
                // The queue item is a `Stopper`

                queue_row.add_css_class("heading");
                queue_row.add_css_class("dimmed");
                // IDEA: Draw a pause icon in place of the album cover
            }

            queue_row.upcast::<gtk::Widget>()
        });
        let _ = self.list_model.set(model);
    }
}

impl WidgetImpl for QueuePage {}
impl NavigationPageImpl for QueuePage {}
