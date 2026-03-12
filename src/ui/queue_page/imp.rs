use adw::{prelude::*, subclass::prelude::*};
use core::cell::{Cell, OnceCell, RefCell};
use core::mem;
use gtk::CompositeTemplate;
use gtk::{gdk, gio, glib};
use std::rc::Rc;
use std::sync::Arc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, Library, SharedSong};
use crate::player::{PLAYER_TX, PlayerRequest, QueueItem, SharedStopper};
use crate::ui::queue_page::QueueScrollAction;
use crate::ui::{ListRow, QueueItemObject, QueueSubpage};
use crate::ui::{UI_TX, UpdateUI, fallback_song_image};
use crate::util::format_duration_ms;

const NUM_ITEMS_AHEAD: usize = 45;
const NUM_ITEMS_BEHIND: usize = 45;
const ROW_HEIGHT: usize = 55;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/queue_page.ui")]
pub struct QueuePage {
    #[template_child]
    header_normal: TemplateChild<adw::HeaderBar>,
    #[template_child]
    pub shuffle_toggle: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pub repeat_toggle: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    header_selection: TemplateChild<adw::HeaderBar>,
    #[template_child]
    pub selection_toggle: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pub remove_selection: TemplateChild<gtk::Button>,

    pub selection_mode: Rc<Cell<bool>>,

    #[template_child]
    list_box: TemplateChild<gtk::ListBox>,
    #[template_child]
    drag_widget: TemplateChild<gtk::Fixed>,
    #[template_child]
    scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,

    queue_length: Cell<usize>,
    playing_index: Cell<usize>,
    last_repeat_mode: Cell<bool>,
    queue_item_objects: Rc<RefCell<Vec<QueueItemObject>>>,
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
    #[template_callback]
    pub fn handle_exit_selection(&self) {
        self.set_selection_mode(false);
    }
    #[template_callback]
    pub fn handle_remove_selected(&self) {
        let mut selected_items = Vec::new();
        let list_model = self.list_model.get().expect(EXP_INIT);
        for i in (0..list_model.n_items()).rev() {
            let item = (list_model.item(i).and_downcast::<QueueItemObject>()).unwrap();
            if item.selected() {
                selected_items.push(item.index() as usize);
            }
        }
        let _ = (PLAYER_TX.get().expect(EXP_INIT)).send(PlayerRequest::RemoveItems(selected_items));
        self.set_selection_mode(false);
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
            self.scroll_to_pos((model_index * ROW_HEIGHT) as f64);

            #[cfg(debug_assertions)]
            self.model_index_to_queue_discrepancy_check(model_index, index);
        }
    }

    pub fn update_song_queue(&self, queue: &[QueueItem], playing: usize) {
        let queue_length = queue.len();
        let old_queue_length = self.queue_length.replace(queue_length);
        self.view_stack
            .set_visible_child_name(match queue.is_empty() {
                true => "queue_empty",
                false => "song_queue",
            });

        // Exit selection because the model gets reset
        self.set_selection_mode(false);

        self.playing_index.set(playing);
        let Some(list_model) = self.list_model.get() else {
            return;
        };

        let start = playing.saturating_sub(NUM_ITEMS_BEHIND);
        let end = (playing + NUM_ITEMS_AHEAD).min(queue.len());

        // Garbage collection
        if old_queue_length > 0 {
            Library::run_task(LIBRARY_TX.get().expect(EXP_INIT), {
                let queue = queue.to_vec();
                move || {
                    // NOTE: Garbage collection happens before assigning the items, due to
                    // background-loaded artworks otherwise sometimes not getting assigned.
                    // If there are issues with queue artworks in the future, try disabling
                    // garbage collection first, to verify that it is working properly.
                    let short_start = playing.saturating_sub(NUM_ITEMS_BEHIND);
                    let short_end = (playing + NUM_ITEMS_AHEAD).min(queue.len());
                    for (index, song) in queue.iter().enumerate() {
                        let QueueItem::Song(song) = song else {
                            return;
                        };

                        if !(start..end).contains(&index) {
                            song.info().try_unload_thumbnail();
                        }

                        // Keep detailed artworks loaded for a few items ahead and behind
                        match (short_start..short_end).contains(&index) {
                            true => drop(song.info().load_detailed()),
                            false => song.info().try_unload_detailed(),
                        }
                    }
                }
            });
        }

        let last_scroll_pos = self.scrolled_window.vadjustment().value();
        let mut items = Self::items_to_objects(
            queue.iter().enumerate().take(end).skip(start),
            playing,
            // :)
        );

        let repeat_mode = self.repeat_toggle.is_active();
        let last_repeat_mode = self.last_repeat_mode.replace(repeat_mode);
        if repeat_mode && queue_length != 0 {
            let n_items_before = (NUM_ITEMS_BEHIND - (playing - start)).min(queue_length - 1);
            if n_items_before > 0 {
                self.next_scroll_pos.set(QueueScrollAction::Offset(
                    n_items_before as i32, //
                ));
                let from = queue.len() - n_items_before;
                let mut items_before = Self::items_to_objects(
                    queue.iter().enumerate().skip(from.max(playing + 1)),
                    playing,
                );
                mem::swap(&mut items, &mut items_before);
                items.extend(items_before);
            }
            let n_items_after = NUM_ITEMS_AHEAD - (end - playing);
            if n_items_after > 0 {
                let items_after = Self::items_to_objects(
                    queue.iter().enumerate().take(n_items_after.min(playing)),
                    playing,
                );
                items.extend(items_after);
            }
        } else if repeat_mode != last_repeat_mode {
            self.next_scroll_pos.set(QueueScrollAction::Offset(
                -(NUM_ITEMS_BEHIND.saturating_sub(playing) as i32),
            ));
        }

        list_model.splice(0, list_model.n_items(), &items);
        self.queue_item_objects.replace(items);

        match self.next_scroll_pos.take() {
            // Re-applying the scroll position, because it resets when the `list_box` rows change
            QueueScrollAction::Retain => self.scroll_to_pos(last_scroll_pos),
            QueueScrollAction::Offset(offset) => {
                self.scroll_to_pos(last_scroll_pos + (offset * ROW_HEIGHT as i32) as f64);
            }
            QueueScrollAction::ToPlaying => self.scroll_to_item(playing),
        }
    }

    #[inline]
    #[must_use]
    fn items_to_objects<I, 'i>(items_iter: I, playing_index: usize) -> Vec<QueueItemObject>
    where
        I: Iterator<Item = (usize, &'i QueueItem)>,
    {
        items_iter
            .map(|index_item| {
                let q_index = index_item.0;
                match index_item.1 {
                    QueueItem::Song(song) => {
                        Self::new_song(q_index as u32, q_index == playing_index, song)
                    }
                    QueueItem::Stopper(stopper) => Self::new_stopper(q_index as u32, stopper),
                }
            })
            .collect()
    }

    #[inline]
    #[must_use]
    fn new_song(queue_index: u32, is_playing: bool, song: &SharedSong) -> QueueItemObject {
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
    fn new_stopper(queue_index: u32, stopper: &SharedStopper) -> QueueItemObject {
        let queue_item_object = QueueItemObject::new(queue_index, false, None);
        queue_item_object.set_title(stopper.display_name());
        queue_item_object
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
    /// Takes a model index and returns the index to access its queue item
    #[inline]
    #[must_use]
    fn model_index_to_queue(&self, model_index: usize) -> usize {
        let playing_index = self.playing_index.get();
        match self.repeat_toggle.is_active() {
            false => model_index + playing_index - NUM_ITEMS_BEHIND.min(playing_index),
            true => {
                let queue_length = self.queue_length.get();
                assert!(
                    queue_length != 0,
                    "`model_index_to_queue` used on an empty queue"
                );

                // Wrapping over the start of the queue
                let n_items_behind = NUM_ITEMS_BEHIND
                    .min(queue_length - 1)
                    .saturating_sub(playing_index);
                if n_items_behind > 0 {
                    // println!("Wrapping over the start of the queue");
                    if model_index < n_items_behind {
                        return queue_length - n_items_behind + model_index;
                    }
                    if model_index == n_items_behind {
                        return 0;
                    }
                }

                // Non-wrapped items
                let offset_index = model_index + playing_index
                    - NUM_ITEMS_BEHIND.min(playing_index)
                    - n_items_behind;
                if offset_index < queue_length {
                    // println!("Non-wrapped item");
                    return offset_index;
                }

                // Wrapping over the end of the queue
                // println!("Wrapping over the end of the queue");
                offset_index - queue_length
            }
        }
    }

    #[inline]
    fn for_each_row<F: Fn(&ListRow, i32)>(&self, f: F) {
        let mut i = 0;
        while let Some(row) = self.list_box.row_at_index(i).and_downcast_ref::<ListRow>() {
            f(row, i);
            i += 1;
        }
    }

    // #[inline]
    // fn for_each_object<F: Fn(&QueueItemObject, u32)>(&self, f: F) {
    //     let mut i = 0;
    //     let list_model = self.list_model.get().expect(EXP_INIT);
    //     while let Some(row) = list_model.item(i).and_downcast_ref::<QueueItemObject>() {
    //         f(item, i);
    //         i += 1;
    //     }
    // }

    #[inline]
    fn set_selection_mode(&self, selection_mode: bool) {
        self.selection_toggle.set_active(selection_mode);
        self.header_selection.set_visible(selection_mode);
        self.header_normal.set_visible(!selection_mode);
        self.selection_mode.set(selection_mode);
        let model = self.list_model.get().expect(EXP_INIT);
        self.for_each_row(|list_row, index| {
            let list_row = list_row.imp();
            list_row.selection_toggle.set_visible(selection_mode);
            list_row.open_subpage_icon.set_visible(!selection_mode);
            if !selection_mode {
                list_row.set_selected(false);
                (model.item(index as u32).unwrap())
                    .downcast::<QueueItemObject>()
                    .unwrap()
                    .set_selected(false);
            }
        });
    }

    #[inline]
    pub fn assign_artwork(&self, index: usize, artwork: Option<&gdk::Texture>) {
        if let Ok(model_index) = self.queue_index_to_model(index) {
            self.queue_item_objects.borrow()[model_index].set_property("artwork", artwork);

            #[cfg(debug_assertions)]
            self.model_index_to_queue_discrepancy_check(model_index, index);
        }
    }

    #[inline]
    fn setup_model(&self) {
        let model = gio::ListStore::new::<QueueItemObject>();
        let selection_mode = Rc::clone(&self.selection_mode);
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

                queue_row.add_bindings(&[
                    queue_item_object
                        .bind_property("artwork", &queue_row.imp().prefix_image.get(), "paintable")
                        .sync_create()
                        .build(),
                    queue_item_object
                        .bind_property(
                            "selected",
                            &queue_row.imp().selection_toggle.get(),
                            "active",
                        )
                        .sync_create()
                        .build(),
                ]);

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
            let selection_mode = Rc::clone(&selection_mode);
            queue_row.connect_activated(glib::clone!(
                #[weak]
                queue_item_object,
                move |_| match selection_mode.get() {
                    false => (UI_TX.get().expect(EXP_INIT))
                        .send(UpdateUI::OpenQueueSubpage(object_index))
                        .expect(EXP_RX),
                    true => queue_item_object.set_selected(!queue_item_object.selected()),
                }
            ));

            queue_row.upcast::<gtk::Widget>()
        });

        let _ = self.list_model.set(model);
    }
    /// Returns `true` if interaction at a given `start_pos_x`
    /// should drag the queue row, or `false` if not
    #[inline]
    fn should_drag(start_pos_x: f64) -> bool {
        start_pos_x < 65.0
    }
    #[inline]
    fn setup_drag_and_drop(&self) {
        let drag = gtk::GestureDrag::new();
        let drag_row = ListRow::default();
        drag_row.add_css_class("color-menu");
        let drag_container = self.drag_widget.parent().unwrap();
        self.drag_widget.set_cursor_from_name(Some("grabbing"));
        self.drag_widget.put(&drag_row, 0.0, 0.0);

        drag.connect_begin(glib::clone!(
            #[weak(rename_to=list_box)]
            self.list_box,
            #[strong(rename_to=selection_mode)]
            self.selection_mode,
            #[weak]
            drag_row,
            move |gesture_drag, _| if !selection_mode.get() {
                let Some((start_x, start_y)) = gesture_drag.point(None) else {
                    return;
                };
                if !Self::should_drag(start_x) {
                    return;
                }

                // FIX: The cursor does not update until the mouse button is released
                list_box.set_cursor_from_name(Some("grabbing"));

                if let Some(row) = list_box.row_at_y(start_y as i32) {
                    let row = row.downcast_ref::<ListRow>().unwrap();
                    drag_row.copy_from(row);
                    drag_row.set_width_request(row.width());
                    drag_row.set_height_request(row.height());
                } else {
                    drag_row.to_default();
                }
            }
        ));
        drag.connect_update(glib::clone!(
            #[weak(rename_to=queue_page)]
            self,
            #[weak]
            drag_row,
            #[weak]
            drag_container,
            move |gesture_drag, _| if !queue_page.selection_mode.get() {
                let (Some((start_x, start_y)), Some((offset_x, offset_y))) =
                    (gesture_drag.start_point(), gesture_drag.offset())
                else {
                    return;
                };
                if !Self::should_drag(start_x) {
                    return;
                }

                if let Some(to_row_index) = (queue_page.list_box)
                    .row_at_y((start_y + offset_y) as i32)
                    .map(|row| row.index())
                {
                    let from_row_index = (queue_page.list_box)
                        .row_at_y((start_y) as i32)
                        .map(|row| row.index())
                        .unwrap_or_default();
                    queue_page.for_each_row(|list_row, index| {
                        if to_row_index - 1 == index && to_row_index < from_row_index
                            || to_row_index == index && to_row_index > from_row_index
                        {
                            list_row.add_css_class("highlight-top");
                        } else {
                            list_row.remove_css_class("highlight-top");
                        }
                    });
                } else {
                    queue_page.for_each_row(|list_row, _| {
                        list_row.remove_css_class("highlight-top");
                    });
                }

                queue_page.drag_widget.move_(
                    &drag_row,
                    start_x + offset_x,
                    start_y + offset_y - queue_page.scrolled_window.vadjustment().value(),
                );

                // Setting here to only show it after moving the cursor
                // TODO: Is it okay to repeatedly call this?
                drag_container.set_visible(true);
            }
        ));
        drag.connect_end(glib::clone!(
            #[weak(rename_to=queue_page)]
            self,
            #[weak]
            drag_row,
            #[weak]
            drag_container,
            move |gesture_drag, _| if !queue_page.selection_mode.get() {
                let list_box = &queue_page.list_box;
                list_box.set_cursor(None);
                drag_container.set_visible(false);
                drag_row.to_default();
                let start_y = match gesture_drag.start_point() {
                    Some((start_x, start_y)) if Self::should_drag(start_x) => start_y,
                    _ => return,
                };
                let end_y = match gesture_drag.offset() {
                    Some((_, offset_y)) => start_y + offset_y,
                    None => return,
                };
                let Some(from) = list_box.row_at_y(start_y as i32).map(|row| row.index()) else {
                    return;
                };
                let Some(to) = list_box.row_at_y(end_y as i32).map(|row| row.index()) else {
                    return;
                };
                let from_index = queue_page.model_index_to_queue(from as usize);
                let playing_index = queue_page.playing_index.get();
                let playing = (queue_page.queue_index_to_model(playing_index)).unwrap() as i32;
                (queue_page.next_scroll_pos).set(QueueScrollAction::Offset(
                    match playing_index > NUM_ITEMS_BEHIND || queue_page.repeat_toggle.is_active() {
                        false if from < playing && to > playing => 1,
                        true if from > playing && to <= playing => -1,
                        true if from < playing && to >= playing => 1,
                        true if from == playing => from - to,
                        _ => 0,
                    },
                ));
                (PLAYER_TX.get().expect(EXP_INIT))
                    .send(PlayerRequest::Shift(from_index, (to - from) as isize))
                    .expect(EXP_RX);

                queue_page.for_each_row(|list_row, _| {
                    list_row.remove_css_class("highlight-top");
                });
            }
        ));
        self.list_box.add_controller(drag);
    }
    #[inline]
    fn setup_selection_mode(&self) {
        // IDEA: Rating dropdown button for rating multiple songs at once
        // TODO: Exit selection mode by pressing escape
        // FIX: Item selections will likely be lost with each queue update

        let selection_mode = Rc::clone(&self.selection_mode);
        let hold = gtk::GestureLongPress::new();
        hold.connect_pressed(glib::clone!(
            #[weak(rename_to=queue_page)]
            self,
            move |_, x, y| if !selection_mode.get() && !Self::should_drag(x) {
                queue_page.set_selection_mode(true);
                let object_index = queue_page.list_box.row_at_y(y as i32).unwrap().index();
                queue_page.queue_item_objects.borrow()[object_index as usize].set_selected(true);
            }
        ));
        self.list_box.add_controller(hold);
    }

    /// Empties the list model, cancelling any pending background tasks during drop
    #[inline]
    pub fn uninit(&self) {
        self.list_model.get().expect(EXP_INIT).remove_all();
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
        // self.obj().update_repeat(false);

        self.setup_model();
        self.setup_drag_and_drop();
        self.setup_selection_mode();
    }
}

impl WidgetImpl for QueuePage {}
impl NavigationPageImpl for QueuePage {}
