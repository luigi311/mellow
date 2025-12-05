use adw::Application;
use adw::{prelude::*, subclass::prelude::*};
use gio::Settings;
use glib::{Object, clone};
use gtk::{Orientation, gdk, gio, glib};
use std::sync::mpsc;

use crate::APP_ID;
use crate::excuses::{EXP_INIT, INIT_ERR};
use crate::library::LibraryRequest;
use crate::player::PlayerRequest;

mod imp;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements
            gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
            gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    #[must_use]
    pub fn new(
        app: &Application,
        library_tx: mpsc::SyncSender<LibraryRequest>,
        player_tx: mpsc::SyncSender<PlayerRequest>,
    ) -> Self {
        let window: Self = Object::builder().property("application", app).build();
        let imp = window.imp();
        imp.player_tx.set(player_tx).expect(INIT_ERR);
        imp.library_tx.set(library_tx).expect(INIT_ERR);
        imp.css_provider
            .set(gtk::CssProvider::new())
            .expect(INIT_ERR);
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                imp.css_provider.get().expect(EXP_INIT),
                210,
            );
        }
        imp.init_ui_elements();
        window.load_settings();
        window
    }

    fn setup_settings(&self) {
        let settings = Settings::new(APP_ID);
        self.imp().settings.set(settings).expect(INIT_ERR);
    }

    fn setup_actions(&self) {
        let player_actions = gio::SimpleActionGroup::new();
        player_actions.add_action_entries([
            gio::ActionEntry::builder("skip_prev")
                .activate(clone!(
                    #[weak(rename_to=player)]
                    self.imp().main_player.imp(),
                    move |_, _, _| player.handle_skip_prev()
                ))
                .build(),
            gio::ActionEntry::builder("play_pause")
                .activate(clone!(
                    #[weak(rename_to=player)]
                    self.imp().main_player.imp(),
                    move |_, _, _| player.handle_play_pause()
                ))
                .build(),
            gio::ActionEntry::builder("skip_next")
                .activate(clone!(
                    #[weak(rename_to=player)]
                    self.imp().main_player.imp(),
                    move |_, _, _| player.handle_skip_next()
                ))
                .build(),
        ]);
        self.insert_action_group("player", Some(&player_actions));

        let ui_actions = gio::SimpleActionGroup::new();
        ui_actions.add_action_entries([
            gio::ActionEntry::builder("open_sheet")
                .activate(clone!(
                    #[weak(rename_to=ui)]
                    self.imp(),
                    move |_, _, _| ui.open_sheet(true)
                ))
                .build(),
            gio::ActionEntry::builder("close_sheet")
                .activate(clone!(
                    #[weak(rename_to=ui)]
                    self.imp(),
                    move |_, _, _| ui.open_sheet(false)
                ))
                .build(),
            gio::ActionEntry::builder("playing_nav_push")
                .parameter_type(Some(&String::static_variant_type()))
                .activate(clone!(
                    #[weak(rename_to=ui)]
                    self.imp(),
                    move |_, _, tag| {
                        let tag = tag.unwrap().get::<String>().unwrap();
                        ui.playing_navigation_view.push_by_tag(&tag);
                    }
                ))
                .build(),
            gio::ActionEntry::builder("playing_nav_pop")
                .activate(clone!(
                    #[weak(rename_to=ui)]
                    self.imp(),
                    move |_, _, _| {
                        ui.playing_navigation_view.pop();
                    }
                ))
                .build(),
        ]);
        self.insert_action_group("ui", Some(&ui_actions));
    }

    fn settings(&self) -> &Settings {
        self.imp().settings.get().expect(EXP_INIT)
    }

    pub fn save_settings(&self) -> Result<(), glib::BoolError> {
        let width = self.size(Orientation::Horizontal);
        let height = self.size(Orientation::Vertical);
        let settings_page = &self.imp().settings_page;
        let volume = settings_page.volume();
        let gapless = settings_page.gapless();

        self.settings().set_int("window-width", width)?;
        self.settings().set_int("window-height", height)?;
        self.settings().set_double("volume", volume)?;
        self.settings().set_boolean("gapless", gapless)?;

        Ok(())
    }

    pub fn load_settings(&self) {
        let width = self.settings().int("window-width");
        let height = self.settings().int("window-height");

        self.set_default_size(width, height);

        let volume = self.settings().double("volume");
        let gapless = self.settings().boolean("gapless");

        // Slider callback `change_value` doesn't work for `set_value()`,
        // so the volume has to be manually updated before the slider
        let settings_page = &self.imp().settings_page;
        settings_page
            .imp()
            .handle_set_volume(gtk::ScrollType::Jump, volume);

        settings_page.set_volume(volume);
        settings_page.set_gapless(gapless);
    }
}
