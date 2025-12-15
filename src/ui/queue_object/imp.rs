use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::glib;
use std::cell::Cell;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::QueueObject)]
pub struct QueueObject {
    #[property(get, set)]
    index: Cell<u64>,
    // #[property(get, set)]
    // playing: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for QueueObject {
    const NAME: &str = "MellowQueueObject";
    type Type = super::QueueObject;
}

#[glib::derived_properties]
impl ObjectImpl for QueueObject {}
