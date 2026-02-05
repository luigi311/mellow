use adw::{prelude::*, subclass::prelude::*};
use gtk::{gdk, glib};

use crate::excuses::{EXP_INIT, INIT_ERR};

mod imp;

glib::wrapper! {
    pub struct SettingsPage(ObjectSubclass<imp::SettingsPage>)
        @extends adw::PreferencesPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl SettingsPage {
    pub fn init(&self, bottom_bar: gtk::Box, sheet: adw::BottomSheet) {
        let imp = self.imp();
        let _ = imp.css_provider.set(gtk::CssProvider::new());
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                imp.css_provider.get().expect(EXP_INIT),
                210,
            );
        }
        imp.bottom_bar.set(bottom_bar).expect(INIT_ERR);
        imp.sheet.set(sheet).expect(INIT_ERR);
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

    pub fn enable_background_color(&self) {
        self.imp().enable_background_color();
    }
    pub fn disable_background_color(&self) {
        self.imp().disable_background_color();
    }
    pub fn set_background_color(&self, r: f64, g: f64, b: f64) {
        self.imp().set_background_color(r, g, b);
    }
    pub fn set_background_from_artwork(&self, artwork: &gdk::Texture) {
        self.imp().set_background_from_artwork(artwork);
    }
}
