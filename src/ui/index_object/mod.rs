use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct IndexObject(ObjectSubclass<imp::IndexObject>);
}

impl IndexObject {
    pub fn new(index: u64) -> Self {
        Object::builder()
            .property("index", index)
            // TODO
            .build()
    }
}
