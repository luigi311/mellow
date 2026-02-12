use std::cell::Ref;

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

#[derive(Default, Debug, Copy, Clone)]
pub enum StartupQueueChoice {
    #[default]
    RestoreQueue = 0,
    QueueFromSongs = 1,
    QueueFromAlbums = 2,
    QueueFromArtists = 3,
    QueueFromSongsShuffled = 4,
    QueueFromAlbumsShuffled = 5,
    QueueFromArtistsShuffled = 6,
    EmptyQueue = 7,
}
impl From<i32> for StartupQueueChoice {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::RestoreQueue,
            1 => Self::QueueFromSongs,
            2 => Self::QueueFromAlbums,
            3 => Self::QueueFromArtists,
            4 => Self::QueueFromSongsShuffled,
            5 => Self::QueueFromAlbumsShuffled,
            6 => Self::QueueFromArtistsShuffled,
            7 => Self::EmptyQueue,
            n => {
                eprintln!(
                    "WARNING: Value {n} is outside the valid range for `StartupQueueChoice` (default value will be used instead)"
                );
                Self::default()
            }
        }
    }
}

impl SettingsPage {
    pub fn init(
        &self,
        style_manager: adw::StyleManager,
        sheet_content: adw::ToolbarView,
        player_controls: gtk::Box,
        window_content: adw::BottomSheet,
        bottom_bar: gtk::Box,
    ) {
        let imp = self.imp();
        imp.player_controls.set(player_controls).expect(INIT_ERR);
        imp.bottom_bar.set(bottom_bar).expect(INIT_ERR);
        imp.window_content.set(window_content).expect(INIT_ERR);
        imp.sheet_content.set(sheet_content).expect(INIT_ERR);
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

    #[must_use]
    pub fn volume(&self) -> f64 {
        self.imp().volume.value()
    }
    pub fn set_volume(&self, volume: f64) {
        self.imp().volume.set_value(volume);
    }

    #[must_use]
    pub fn gapless(&self) -> bool {
        self.imp().gapless.is_active()
    }
    pub fn set_gapless(&self, gapless: bool) {
        self.imp().gapless.set_active(gapless);
    }

    #[must_use]
    pub fn startup_queue(&self) -> Ref<'_, StartupQueueChoice> {
        self.imp().startup_choice.borrow()
    }
    pub fn set_startup_queue(&self, choice: StartupQueueChoice) {
        let settings = self.imp();
        settings.startup_choice.replace(choice);
        match choice {
            StartupQueueChoice::RestoreQueue => settings.remember_queue.set_active(true),
            StartupQueueChoice::EmptyQueue => settings.empty_queue.set_active(true),
            _ => {
                settings.new_queue.set_active(true);
                settings.shuffle_queue.set_active(choice as i32 > 3);
                settings.queue_source.set_selected(match choice as u32 - 1 {
                    source if source > 2 => source - 3,
                    source => source,
                });
            }
        }
    }

    #[must_use]
    pub fn remembers_queue(&self) -> bool {
        matches!(
            *self.imp().startup_choice.borrow(),
            StartupQueueChoice::RestoreQueue
        )
    }
    #[must_use]
    pub fn remembers_time(&self) -> bool {
        self.imp().remember_time.is_active()
    }
    pub fn set_remember_time(&self, remember_time: bool) {
        self.imp().remember_time.set_active(remember_time);
    }

    #[must_use]
    pub fn adaptive_colors(&self) -> bool {
        self.imp().adaptive_colors.is_active()
    }
    pub fn set_adaptive_colors(&self, adaptive_colors: bool) {
        self.imp().adaptive_colors.set_active(adaptive_colors);
    }

    #[must_use]
    pub fn color_scheme(&self) -> u32 {
        self.imp().color_scheme.selected()
    }
    pub fn set_color_scheme(&self, id: u32) {
        self.imp().color_scheme.set_selected(id);
    }

    #[must_use]
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
