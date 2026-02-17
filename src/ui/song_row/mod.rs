use adw::subclass::prelude::*;
use glib::{Object, object::IsA};
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct SongRow(ObjectSubclass<imp::SongRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for SongRow {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl SongRow {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_prefix_image(&self, image: Option<&impl IsA<gdk::Paintable>>) {
        self.imp().prefix_image.set_paintable(image);
    }

    pub fn set_suffix_label(&self, content: &str) {
        self.imp().suffix_label.set_label(content);
    }

    pub fn add_bindings(&self, bindings: &[glib::Binding]) {
        self.imp().bindings.borrow_mut().extend_from_slice(bindings);
    }
    pub fn reset_bindings(&self) {
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
