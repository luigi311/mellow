use adw::subclass::prelude::*;
use gtk::{glib, prelude::RangeExt};

mod imp;

glib::wrapper! {
    pub struct SettingsPage(ObjectSubclass<imp::SettingsPage>)
        @extends adw::PreferencesPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl SettingsPage {
    pub fn volume(&self) -> f64 {
        self.imp().volume.value()
    }
    pub fn set_volume(&self, volume: f64) {
        self.imp().volume.set_value(volume);
    }

    pub fn gapless(&self) -> bool {
        self.imp().gapless.is_active()
    }
    pub fn set_gapless(&self, gapless: bool) {
        self.imp().gapless.set_active(gapless);
    }

    pub fn remembers_queue(&self) -> bool {
        self.imp().remember_queue.is_active()
    }
    pub fn set_remember_queue(&self, remember_queue: bool) {
        self.imp().remember_queue.set_active(remember_queue);
    }

    pub fn directories(&self) -> Vec<String> {
        self.imp().directories.borrow().clone()
    }
    pub fn set_directories(&self, directories: &[String]) {
        self.imp().set_directories(directories);
    }
}
