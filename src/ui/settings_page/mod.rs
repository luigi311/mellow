use adw::subclass::prelude::*;
use glib::Object;
use gtk::{glib, prelude::RangeExt};
use std::sync::mpsc;

use crate::excuses::INIT_ERR;
use crate::library::LibraryRequest;
use crate::player::PlayerRequest;

mod imp;

glib::wrapper! {
    pub struct SettingsPage(ObjectSubclass<imp::SettingsPage>)
        @extends adw::PreferencesPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for SettingsPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl SettingsPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn init(
        &self,
        player_tx: mpsc::SyncSender<PlayerRequest>,
        library_tx: mpsc::SyncSender<LibraryRequest>,
    ) {
        self.imp().player_tx.set(player_tx).expect(INIT_ERR);
        self.imp().library_tx.set(library_tx).expect(INIT_ERR);
    }

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

    pub fn set_directories(&self, directories: &[String]) {
        self.imp().set_directories(directories);
    }
}
