use adw::subclass::prelude::*;
use glib::Object;
use gst::glib::object::IsA;
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct QueueRow(ObjectSubclass<imp::QueueRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl QueueRow {
    pub fn new() -> Self {
        Object::builder()
            // TODO
            .build()
    }

    pub fn set_prefix_image(&self, image: &impl IsA<gdk::Paintable>) {
        self.imp().prefix_image.set_paintable(Some(image));
    }
}
