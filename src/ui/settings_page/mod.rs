use adw::{prelude::*, subclass::prelude::*};
use gtk::{gdk, glib};

use crate::excuses::INIT_ERR;

mod imp;

glib::wrapper! {
    pub struct SettingsPage(ObjectSubclass<imp::SettingsPage>)
        @extends adw::PreferencesPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl SettingsPage {
    pub fn init(
        &self,
        style_manager: adw::StyleManager,
        bottom_bar: gtk::Box,
        sheet: adw::BottomSheet,
    ) {
        let imp = self.imp();
        imp.bottom_bar.set(bottom_bar).expect(INIT_ERR);
        imp.sheet.set(sheet).expect(INIT_ERR);
        // TODO: Detect color cheme
        // let style_preference = style_manager.color_scheme();
        let _ = imp.css.set(gtk::CssProvider::new());
        let css = imp.css.get().expect(INIT_ERR);
        imp.style_manager.set(style_manager).expect(INIT_ERR);
        imp.set_theme(adw::ColorScheme::ForceDark);
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, css, 210);
        }
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
    pub fn reset_background_color(&self) {
        self.imp().reset_background_color();
    }

    pub fn set_background_color(&self, r: f64, g: f64, b: f64) {
        self.imp().set_background_color(r, g, b);
    }
    pub fn set_background_from_artwork(&self, artwork: &gdk::Texture) {
        self.imp().set_background_from_artwork(artwork);
    }
}
