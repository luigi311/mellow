mod imp;

use adw::Application;
use adw::{prelude::*, subclass::prelude::*};
use gio::Settings;
use glib::Object;
use gtk::{Orientation, gio, glib};

use std::sync::mpsc;

use crate::APP_ID;
use crate::player::PlayerRequest;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements
            gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
            gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    #[must_use]
    pub fn new(app: &Application) -> Self {
        Object::builder().property("application", app).build()
    }

    pub fn register_player_tx(&self, player_tx: mpsc::SyncSender<PlayerRequest>) {
        self.imp().player_tx.set(player_tx).unwrap();
    }

    fn setup_settings(&self) {
        let settings = Settings::new(APP_ID);
        self.imp().settings.set(settings).unwrap();
    }

    fn settings(&self) -> &Settings {
        self.imp().settings.get().unwrap()
    }

    pub fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let width = self.size(Orientation::Horizontal);
        let height = self.size(Orientation::Vertical);

        self.settings().set_int("window-width", width)?;
        self.settings().set_int("window-height", height)?;

        Ok(())
    }

    pub fn load_window_size(&self) {
        let width = self.settings().int("window-width");
        let height = self.settings().int("window-height");

        self.set_default_size(width, height);
    }
}
