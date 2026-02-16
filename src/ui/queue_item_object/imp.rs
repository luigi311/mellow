use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::{gdk, glib};

use std::cell::{OnceCell, RefCell};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{library::song::SharedSong, ui::queue_item_object::QueueItemData};

#[derive(Properties, Default)]
#[properties(wrapper_type = super::QueueItemObject)]
pub struct QueueItemObject {
    #[property(name = "index", get, set, type = u32, member = index)]
    #[property(name = "playing", get, set, type = bool, member = playing)]
    #[property(name = "title", get, set, type = String, member = title)]
    #[property(name = "subtitle", get, set, type = String, member = subtitle)]
    #[property(name = "artwork", get, set, type = Option<gdk::Texture>, member = artwork)]
    pub data: RefCell<QueueItemData>,

    pub shared_song: OnceCell<Option<SharedSong>>,
    pub is_visible: Arc<AtomicBool>,
}

#[glib::object_subclass]
impl ObjectSubclass for QueueItemObject {
    const NAME: &str = "MellowQueueItemObject";
    type Type = super::QueueItemObject;
}

#[glib::derived_properties]
impl ObjectImpl for QueueItemObject {}

impl Drop for QueueItemObject {
    fn drop(&mut self) {
        self.is_visible.store(false, Ordering::Relaxed);
    }
}
