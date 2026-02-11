use adw::Application;
use adw::{prelude::*, subclass::prelude::*};
use core::error::Error;
use gdk::{DragAction, FileList};
use gio::Settings;
use glib::{Object, clone};
use gtk::{Orientation, gdk, gio, glib};
use std::sync::mpsc;
use std::time::Duration;

use crate::about;
use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, Library, LibraryRequest};
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::serializer::serialize_list;

mod imp;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements
            gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
            gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    #[inline]
    #[must_use]
    pub fn new(app: &Application, settings: Settings) -> Self {
        let window: Self = Object::builder().property("application", app).build();
        let imp = window.imp();
        let _ = imp.settings.set(settings);
        imp.init_ui_elements(app.style_manager());
        window.load_state();
        window
    }

    fn settings(&self) -> &Settings {
        self.imp().settings.get().expect(EXP_INIT)
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
            gio::ActionEntry::builder("play_all_songs")
                .activate(clone!(
                    #[weak(rename_to=songs_page)]
                    self.imp().songs_page.imp(),
                    move |_, _, _| songs_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_all_songs")
                .activate(clone!(
                    #[weak(rename_to=songs_page)]
                    self.imp().songs_page.imp(),
                    move |_, _, _| songs_page.handle_play_shuffled()
                ))
                .build(),
            gio::ActionEntry::builder("play_visible_album")
                .activate(clone!(
                    #[weak(rename_to=album_page)]
                    self.imp().album_page.imp(),
                    move |_, _, _| album_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_visible_album")
                .activate(clone!(
                    #[weak(rename_to=album_page)]
                    self.imp().album_page.imp(),
                    move |_, _, _| album_page.handle_play_shuffled()
                ))
                .build(),
            gio::ActionEntry::builder("play_visible_artist")
                .activate(clone!(
                    #[weak(rename_to=artist_page)]
                    self.imp().artist_page.imp(),
                    move |_, _, _| artist_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_visible_artist")
                .activate(clone!(
                    #[weak(rename_to=artist_page)]
                    self.imp().artist_page.imp(),
                    move |_, _, _| artist_page.handle_play_shuffled()
                ))
                .build(),
            gio::ActionEntry::builder("play_all_albums")
                .activate(clone!(
                    #[weak(rename_to=albums_page)]
                    self.imp().albums_page.imp(),
                    move |_, _, _| albums_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_all_albums")
                .activate(clone!(
                    #[weak(rename_to=albums_page)]
                    self.imp().albums_page.imp(),
                    move |_, _, _| albums_page.handle_play_shuffled()
                ))
                .build(),
            gio::ActionEntry::builder("play_all_artists")
                .activate(clone!(
                    #[weak(rename_to=artists_page)]
                    self.imp().artists_page.imp(),
                    move |_, _, _| artists_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_all_artists")
                .activate(clone!(
                    #[weak(rename_to=artists_page)]
                    self.imp().artists_page.imp(),
                    move |_, _, _| artists_page.handle_play_shuffled()
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
                        ui.playing.push_by_tag(&tag);
                    }
                ))
                .build(),
            gio::ActionEntry::builder("playing_nav_pop")
                .activate(clone!(
                    #[weak(rename_to=ui)]
                    self.imp(),
                    move |_, _, _| {
                        ui.playing.pop();
                    }
                ))
                .build(),
            gio::ActionEntry::builder("library_nav_push")
                .parameter_type(Some(&String::static_variant_type()))
                .activate(clone!(
                    #[weak(rename_to=ui)]
                    self.imp(),
                    move |_, _, tag| {
                        let tag = tag.unwrap().get::<String>().unwrap();
                        ui.library.push_by_tag(&tag);
                    }
                ))
                .build(),
            gio::ActionEntry::builder("library_nav_pop")
                .activate(clone!(
                    #[weak(rename_to=ui)]
                    self.imp(),
                    move |_, _, _| {
                        ui.library.pop();
                    }
                ))
                .build(),
        ]);
        self.insert_action_group("ui", Some(&ui_actions));

        self.add_action_entries([gio::ActionEntry::builder("show_about_dialog")
            .activate(clone!(move |window: &Window, _, _| {
                about::show_about_dialog(window);
            }))
            .build()]);
    }

    pub fn setup_drag_and_drop(&self) {
        let drop_target =
            gtk::DropTarget::new(FileList::static_type(), DragAction::COPY | DragAction::MOVE);
        // TODO: Add visual feedback when the file is over the window
        drop_target.connect_drop(|_, value, _, _| {
            let files: Vec<String> = value
                .get::<FileList>()
                .unwrap()
                .files()
                .iter()
                .map(|file| file.path().unwrap().to_str().unwrap().to_string())
                .collect();
            LIBRARY_TX
                .get()
                .expect(EXP_INIT)
                .send(LibraryRequest::QueueFromPaths(files.into()))
                .expect(EXP_RX);
            true
        });
        self.add_controller(drop_target);
    }

    /// Saves all settings and the player state
    /// Note that `song_queue` will be uninitialized
    pub fn save_state(&self) -> Result<(), Box<dyn Error>> {
        let imp = self.imp();
        let settings_page = &imp.settings_page;
        let remember_queue = settings_page.remembers_queue();

        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let (library_shutdown_tx, library_shutdown_rx) = mpsc::channel();
        Library::run_task(library_tx, move || {
            let player_tx = PLAYER_TX.get().expect(EXP_INIT);
            let (player_shutdown_tx, player_shutdown_rx) = mpsc::channel();
            player_tx
                .send(PlayerRequest::Shutdown(remember_queue, player_shutdown_tx))
                .expect(EXP_RX);
            let _ = player_shutdown_rx.recv();

            library_tx
                .send(LibraryRequest::Shutdown(library_shutdown_tx))
                .expect(EXP_RX);
        });

        let settings = self.settings();
        settings.set_int("window-width", self.size(Orientation::Horizontal))?;
        settings.set_int("window-height", self.size(Orientation::Vertical))?;
        settings.set_double("volume", settings_page.volume())?;
        settings.set_boolean("gapless", settings_page.gapless())?;
        settings.set_boolean("remember-queue", remember_queue)?;
        settings.set_boolean("adaptive-colors", settings_page.adaptive_colors())?;
        settings.set_enum("color-scheme", settings_page.color_scheme().cast_signed())?;
        settings.set_string("directories", &serialize_list(&settings_page.directories()))?;

        library_shutdown_rx.recv_timeout(Duration::from_millis(1500))?;
        Ok(())
    }

    pub fn load_state(&self) {
        let settings = self.settings();
        let settings_page = &self.imp().settings_page;

        // Slider callback `change_value` doesn't work for `set_value()`,
        // so the volume has to be set manually before setting the slider
        let volume = settings.double("volume");
        settings_page
            .imp()
            .handle_set_volume(gtk::ScrollType::Jump, volume);
        settings_page.set_volume(volume);
        settings_page.set_gapless(settings.boolean("gapless"));
        settings_page.set_remember_queue(settings.boolean("remember-queue"));
        settings_page.set_adaptive_colors(settings.boolean("adaptive-colors"));
        settings_page.set_color_scheme(settings.enum_("color-scheme").cast_unsigned());

        self.set_default_size(settings.int("window-width"), settings.int("window-height"));
    }
}
