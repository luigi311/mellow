use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct QueueObject(ObjectSubclass<imp::QueueObject>);
}

impl QueueObject {
    pub fn new() -> Self {
        Object::builder()
            // TODO
            .build()
    }
}
