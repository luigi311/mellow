use adw::{prelude::*, subclass::prelude::*};
use core::cell::{Cell, OnceCell, RefCell};
use core::mem;
use gtk::CompositeTemplate;
use gtk::{gdk, gio, glib};
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::format_duration_ms;
use crate::library::{LIBRARY_TX, Library, SharedSong};
use crate::player::{PLAYER_TX, PlayerRequest, QueueItem, SharedStopper};
use crate::ui::queue_page::QueueScrollAction;
use crate::ui::{ListRow, QueueItemObject, QueueSubpage};
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

    queue_length: Cell<usize>,
    playing_index: Cell<usize>,
    queue_item_objects: RefCell<Vec<QueueItemObject>>,
    pub song_page: OnceCell<QueueSubpage>,
    list_model: OnceCell<gio::ListStore>,
    pub next_scroll_pos: Cell<QueueScrollAction>,
}

#[derive(Debug)]
struct ItemNotFoundError;

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
    pub fn scroll_to_pos(&self, scroll_target: f64) {
        let scrolled_window = self.scrolled_window.get();
        // WORKAROUND: Setting the scroll position in an idle task because it
        // doesn't update otherwise
        glib::idle_add_local(move || {
            scrolled_window.vadjustment().set_value(scroll_target);
            glib::ControlFlow::Break
        });
    }

    #[inline]
    pub fn scroll_to_item(&self, index: usize) {
        if let Ok(model_index) = self.queue_index_to_model(index) {
            self.scroll_to_pos((model_index * 54) as f64);

            #[cfg(debug_assertions)]
            self.model_index_to_queue_discrepancy_check(model_index, index);
        }
    }

    pub fn update_song_queue(&self, queue: &[QueueItem], index: usize) {
        let queue_length = queue.len();
        self.queue_length.set(queue_length);
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
                // NOTE: Garbage collection happens before assigning the items, due to
                // background-loaded artworks otherwise sometimes not getting assigned.
                // If there are issues with queue artworks in the future, try disabling
                // garbage collection first, to verify that it is working properly.
                let short_start = index.saturating_sub(NUM_ITEMS_BEHIND);
                let short_end = (index + NUM_ITEMS_AHEAD).min(queue.len());
                for (index, song) in queue.iter().enumerate() {
                    let QueueItem::Song(song) = song else {
                        return;
                    };
                    if !(start..end).contains(&index)
                        && let Ok(thumbnail) = song.info().try_inspect_thumbnail().as_ref()
                        && thumbnail
                            .as_ref()
                            .is_some_and(|artwork| artwork.ref_count() < 2)
                    {
                        song.info().unload_thumbnail();
                    }

                    // Keep detailed artworks loaded for a few items ahead and behind
                    if (short_start..short_end).contains(&index) {
                        drop(song.info().load_detailed());
                    } else {
                        song.info().try_unload_detailed();
                    }
                }
            }
        });

        let previous_scroll_position = self.scrolled_window.vadjustment().value();
        let mut items: Vec<QueueItemObject> = (queue.iter().enumerate().take(end).skip(start))
            .map(|index_item| {
                let q_index = index_item.0;
                match index_item.1 {
                    QueueItem::Song(song) => self.new_song(q_index as u32, q_index == index, song),
                    QueueItem::Stopper(stopper) => self.new_stopper(q_index as u32, stopper),
                }
            })
            .collect();

        if self.repeat_toggle.is_active() && queue_length != 0 {
            let n_items_before = (NUM_ITEMS_BEHIND - (index - start)).min(queue_length - 1);
            if n_items_before > 0 {
                let from = queue.len() - n_items_before;
                let mut items_before: Vec<QueueItemObject> = (queue
                    .iter()
                    .enumerate()
                    .skip(from.max(index + 1)))
                .map(|index_item| {
                    let q_index = index_item.0;
                    match index_item.1 {
                        QueueItem::Song(song) => {
                            self.new_song(q_index as u32, q_index == index, song)
                        }
                        QueueItem::Stopper(stopper) => self.new_stopper(q_index as u32, stopper),
                    }
                })
                .collect();
                mem::swap(&mut items, &mut items_before);
                items.extend(items_before);
            }
            let n_items_after = NUM_ITEMS_AHEAD - (end - index);
            if n_items_after > 0 {
                let items_after: Vec<QueueItemObject> = (queue
                    .iter()
                    .enumerate()
                    .take(n_items_after.min(index)))
                .map(|index_item| {
                    let q_index = index_item.0;
                    match index_item.1 {
                        QueueItem::Song(song) => {
                            self.new_song(q_index as u32, q_index == index, song)
                        }
                        QueueItem::Stopper(stopper) => self.new_stopper(q_index as u32, stopper),
                    }
                })
                .collect();
                items.extend(items_after);
            }
        }

        list_model.splice(0, list_model.n_items(), &items);
        self.queue_item_objects.replace(items);

        match self.next_scroll_pos.take() {
            // Re-applying the scroll position, because it resets when the `list_box` rows change
            QueueScrollAction::Retain => self.scroll_to_pos(previous_scroll_position),
            QueueScrollAction::ToPlaying => self.scroll_to_item(index),
        }
    }

    #[inline]
    #[must_use]
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

        if let Ok(thumbnail) = info.try_inspect_thumbnail()
            && let Some(thumbnail) = thumbnail.as_ref()
        {
            object.set_artwork(thumbnail);
        }

        object
    }

    #[inline]
    #[must_use]
    fn new_stopper(&self, queue_index: u32, stopper: &SharedStopper) -> QueueItemObject {
        let queue_item_object = QueueItemObject::new(queue_index, false, None);
        queue_item_object.set_title(stopper.display_name());
        queue_item_object
    }

    #[inline]
    pub fn assign_artwork(&self, index: usize, artwork: Option<&gdk::Texture>) {
        if let Ok(model_index) = self.queue_index_to_model(index) {
            self.queue_item_objects.borrow()[model_index].set_property("artwork", artwork);

            #[cfg(debug_assertions)]
            self.model_index_to_queue_discrepancy_check(model_index, index);
        }
    }

    /// Takes a queue item index and returns the index to access its object
    ///
    /// # Panics
    /// Panics if `self.queue_item_objects` `RefCell` is mutably borrowed
    #[inline]
    fn queue_index_to_model(&self, queue_index: usize) -> Result<usize, ItemNotFoundError> {
        let queue_items_len = self.queue_item_objects.borrow().len();
        let playing_index = self.playing_index.get();
        match self.repeat_toggle.is_active() {
            false => {
                let start = playing_index.saturating_sub(NUM_ITEMS_BEHIND);
                if queue_index < start {
                    return Err(ItemNotFoundError);
                }
                match queue_index + NUM_ITEMS_BEHIND.min(playing_index) - playing_index {
                    value if value >= queue_items_len => Err(ItemNotFoundError),
                    value => Ok(value),
                }
            }
            true => {
                let queue_length = self.queue_length.get();
                if queue_length == 0 {
                    return Err(ItemNotFoundError);
                }

                let start = playing_index.saturating_sub(NUM_ITEMS_BEHIND);
                let n_items_before = NUM_ITEMS_BEHIND
                    .min(queue_length - 1 - start)
                    .saturating_sub(playing_index - start);

                // Wrapping over the start of the queue
                if n_items_before > 0 && queue_index > playing_index + NUM_ITEMS_AHEAD {
                    let from = queue_length - n_items_before;
                    match queue_index - from {
                        value if value >= queue_items_len => return Err(ItemNotFoundError),
                        value => return Ok(value),
                    };
                }

                // Non-wrapped items
                if let Some(value) = (queue_index + NUM_ITEMS_BEHIND.min(playing_index))
                    .checked_sub(playing_index)
                    .map(|i| i + n_items_before)
                    && value < queue_items_len
                {
                    return Ok(value);
                }

                // Wrapping over the end of the queue
                let n_items_after = queue_length - playing_index.saturating_sub(NUM_ITEMS_AHEAD);
                if queue_index <= n_items_after {
                    match queue_index + n_items_after {
                        value if value >= queue_items_len => return Err(ItemNotFoundError),
                        value => return Ok(value),
                    }
                }

                Err(ItemNotFoundError)
            }
        }
    }
    // /// Takes a model index and returns the index to access its queue item
    #[inline]
    #[must_use]
    fn model_index_to_queue(&self, model_index: usize) -> usize {
        // TODO: Use this function to shift elements once drag-&-drop works
        let playing_index = self.playing_index.get();
        match self.repeat_toggle.is_active() {
            false => model_index + playing_index - NUM_ITEMS_BEHIND.min(playing_index),
            true => {
                let queue_length = self.queue_length.get();
                assert!(
                    queue_length != 0,
                    "`model_index_to_queue` used on an empty queue"
                );

                let n_items_behind = NUM_ITEMS_AHEAD.saturating_sub(playing_index);

                // Wrapping over the start of the queue
                if n_items_behind > 0 && n_items_behind <= NUM_ITEMS_BEHIND {
                    return queue_length - n_items_behind + model_index;
                }

                // Non-wrapped items
                let offset_index = model_index + playing_index;
                if offset_index < queue_length {
                    return offset_index - NUM_ITEMS_BEHIND.min(playing_index);
                }

                // Wrapping over the end of the queue
                offset_index - queue_length - NUM_ITEMS_BEHIND.min(playing_index)
            }
        }
    }

    /// Used to verify that `model_index_to_queue` is working correctly
    #[inline]
    #[allow(unused)]
    #[cfg(debug_assertions)]
    fn model_index_to_queue_discrepancy_check(&self, model_index: usize, expected_index: usize) {
        match self.model_index_to_queue(model_index) {
            to_queue_index if to_queue_index != expected_index => {
                eprintln!("Discrepancy between `queue_index_to_model` and `model_index_to_queue`:");
                eprintln!("	`queue_index_to_model({expected_index})`:	{model_index}");
                eprintln!("	`model_index_to_queue({model_index})`:	{to_queue_index}");
            }
            _ => (),
        }
    }
    /// Used to verify that `queue_index_to_model` is working correctly
    #[inline]
    #[allow(unused)]
    #[cfg(debug_assertions)]
    fn queue_index_to_model_discrepancy_check(&self, queue_index: usize, expected_index: usize) {
        match self.queue_index_to_model(queue_index) {
            Ok(to_model_index) if to_model_index != expected_index => {
                eprintln!("Discrepancy between `queue_index_to_model` and `model_index_to_queue`:");
                eprintln!("	`model_index_to_queue({expected_index})`:	{queue_index}");
                eprintln!("	`queue_index_to_model({queue_index})`:	{to_model_index}");
            }
            Err(_) => eprintln!("`queue_index_to_model({queue_index})` returned an error"),
            _ => (),
        }
    }

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

        self.next_scroll_pos.set(QueueScrollAction::ToPlaying);
        let model = gio::ListStore::new::<QueueItemObject>();
        self.list_box.bind_model(Some(&model), move |object| {
            let queue_item_object = object.downcast_ref::<QueueItemObject>().unwrap();

            let queue_row = ListRow::default();
            queue_row.set_title(&queue_item_object.title());
            queue_row.set_subtitle(&queue_item_object.subtitle());

            if queue_item_object.shared_song().is_some() {
                // The queue item is a `Song`

                if queue_item_object.playing() {
                    queue_row.add_css_class("heading");
                    queue_row.add_css_class("card");
                    queue_row.set_image_margins(5);
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
            } else {
                // The queue item is a `Stopper`

                queue_row.add_css_class("heading");
                queue_row.add_css_class("dimmed");
                // IDEA: Draw a pause icon in place of the album cover
            }

            let object_index = queue_item_object.index() as usize;
            queue_row.connect_activated(move |_| {
                (UI_TX.get().expect(EXP_INIT))
                    .send(UpdateUI::OpenQueueSubpage(object_index))
                    .expect(EXP_RX);
            });

            queue_row.upcast::<gtk::Widget>()
        });
        let _ = self.list_model.set(model);
    }
}

impl WidgetImpl for QueuePage {}
impl NavigationPageImpl for QueuePage {}
