//! This was made for learning/experimenting with `GObject`s,
//! and can be removed. Keeping it for now, just in case it
//! might come useful while prototyping list implementations.

use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct IndexObject(ObjectSubclass<imp::IndexObject>);
}

impl IndexObject {
    pub fn new(index: u64) -> Self {
        Object::builder().property("index", index).build()
    }
}
