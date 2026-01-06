use adw::subclass::prelude::*;
use glib::{Object, object::IsA};
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct QueueRow(ObjectSubclass<imp::QueueRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for QueueRow {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl QueueRow {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_prefix_image(&self, image: Option<&impl IsA<gdk::Paintable>>) {
        self.imp().prefix_image.set_paintable(image);
    }
}
