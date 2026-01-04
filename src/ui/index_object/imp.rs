use adw::{prelude::*, subclass::prelude::*};
use glib::Properties;
use gtk::glib;
use std::cell::Cell;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::IndexObject)]
pub struct IndexObject {
    #[property(get, set)]
    index: Cell<u64>,
}

#[glib::object_subclass]
impl ObjectSubclass for IndexObject {
    const NAME: &str = "MellowIndexObject";
    type Type = super::IndexObject;
}

#[glib::derived_properties]
impl ObjectImpl for IndexObject {}
