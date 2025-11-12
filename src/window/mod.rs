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
    pub fn new(app: &Application, player_tx: mpsc::SyncSender<PlayerRequest>) -> Self {
        let window: Self = Object::builder().property("application", app).build();
        window.imp().player_tx.set(player_tx).unwrap();
        window
    }

    fn setup_settings(&self) {
        let settings = Settings::new(APP_ID);
        self.imp().settings.set(settings).unwrap();
    }

    fn settings(&self) -> &Settings {
        self.imp().settings.get().unwrap()
    }

    pub fn save_settings(&self) -> Result<(), glib::BoolError> {
        let width = self.size(Orientation::Horizontal);
        let height = self.size(Orientation::Vertical);
        let volume = self.imp().settings_volume.value();
        let gapless = self.imp().settings_gapless.is_active();

        self.settings().set_int("window-width", width)?;
        self.settings().set_int("window-height", height)?;
        self.settings().set_double("volume", volume)?;
        self.settings().set_boolean("gapless", gapless)?;

        Ok(())
    }

    pub fn load_settings(&self) {
        let width = self.settings().int("window-width");
        let height = self.settings().int("window-height");
        let volume = self.settings().double("volume");
        let gapless = self.settings().boolean("gapless");

        // Slider callback `change_value` doesn't work for `set_value()`,
        // so the volume has to be manually updated before being set
        self.imp().handle_set_volume(gtk::ScrollType::Jump, volume);

        self.set_default_size(width, height);
        self.imp().settings_volume.set_value(volume);
        self.imp().settings_gapless.set_active(gapless);
    }
}
