use adw::{prelude::*, subclass::prelude::*};
use glib::{Object, object::IsA};
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct ListRow(ObjectSubclass<imp::ListRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for ListRow {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ListRow {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_prefix_image(&self, image: Option<&impl IsA<gdk::Paintable>>) {
        self.imp().prefix_image.set_paintable(image);
    }

    #[must_use]
    pub fn get_paintable(&self) -> Option<gdk::Paintable> {
        self.imp().prefix_image.paintable()
    }

    pub fn set_suffix_label(&self, content: &str) {
        self.imp().suffix_label.set_label(content);
    }

    pub fn set_image_margins(&self, margin: i32) {
        let row = self.imp();
        row.prefix_image.set_margin_top(margin);
        row.prefix_image.set_margin_bottom(margin);
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
