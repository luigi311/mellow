use adw::Application;
use adw::{prelude::*, subclass::prelude::*};
use core::error::Error;
use gdk::{DragAction, FileList};
use gio::Settings;
use glib::Object;
use gtk::{Orientation, gdk, gio, glib};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::about;
use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::config::LibraryConfig;
use crate::library::{LIBRARY_TX, LibraryRequest};
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::serializer::serialize_list;
use crate::ui::{UI_TX, UpdateUI, actions};

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

    #[inline]
    fn settings(&self) -> &Settings {
        self.imp().settings.get().expect(EXP_INIT)
    }

    fn setup_actions(&self) {
        let player_actions = gio::SimpleActionGroup::new();
        player_actions.add_action_entries([
            actions::player::skip_prev(self),
            actions::player::play_pause(self),
            actions::player::skip_next(self),
            actions::player::play_all_songs(self),
            actions::player::play_all_albums(self),
            actions::player::play_all_artists(self),
            actions::player::play_visible_album(self),
            actions::player::shuffle_visible_album(self),
            actions::player::play_visible_artist(self),
            actions::player::shuffle_visible_artist(self),
        ]);
        self.insert_action_group("player", Some(&player_actions));

        let ui_actions = gio::SimpleActionGroup::new();
        ui_actions.add_action_entries([
            actions::ui::open_sheet(self),
            actions::ui::close_sheet(self),
            actions::ui::playing_nav_push(self),
            actions::ui::playing_nav_pop(self),
            actions::ui::library_nav_pop(self),
        ]);
        self.insert_action_group("ui", Some(&ui_actions));

        // IDEA: Song/album/artist pages could be constructed anew every time they are
        // pushed onto the view stack, and an `Rc<RefCell<Vec>>` variable would keep
        // track of their order. For example, to change the button shuffle behavior on
        // a subpage, simply grab the top-most one in the `Vec`, and call the function.
        // This might work for implementing the go-to-album/artist functionality

        let window = self.imp();
        let menu_actions = gio::SimpleActionGroup::new();
        menu_actions.add_action_entries([
            actions::menu::songs_sort_mode(window.songs_page.get()),
            actions::menu::albums_sort_mode(window.albums_page.get()),
            actions::menu::artists_sort_mode(window.artists_page.get()),
            actions::menu::songs_play_mode(window.songs_page.get()),
            actions::menu::albums_play_mode(window.albums_page.get()),
            actions::menu::artists_play_mode(window.artists_page.get()),
            actions::menu::album_page_play_mode(Rc::clone(&window.album_pages)),
            actions::menu::artist_page_play_mode(Rc::clone(&window.artist_pages)),
        ]);
        self.insert_action_group("menu", Some(&menu_actions));

        self.add_action_entries([gio::ActionEntry::builder("show_about_dialog")
            .activate(move |window: &Window, _, _| about::show_about_dialog(window))
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
                .map(|file| file.path().unwrap().to_str().unwrap().to_owned())
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

    /// Saves all settings and the player state and prepares
    /// for shutdown, uninitializing various components
    pub fn save_and_uninit(&self) -> Result<(), Box<dyn Error>> {
        let _ = UI_TX.get().expect(EXP_INIT).send(UpdateUI::Shutdown);

        let imp = self.imp();
        let settings_page = &imp.settings_page;
        let remember_queue = settings_page.remembers_queue();
        let remember_time = settings_page.remembers_time();
        let (library_shutdown_tx, library_shutdown_rx) = mpsc::channel();

        thread::spawn(move || {
            LibraryConfig::config_dir_create_if_missing();

            let (player_shutdown_tx, player_shutdown_rx) = mpsc::channel();
            (PLAYER_TX.get().expect(EXP_INIT))
                .send(PlayerRequest::Shutdown(
                    remember_queue,
                    remember_time,
                    player_shutdown_tx,
                ))
                .expect(EXP_RX);

            // Wait for the player shutdown request to be processed
            // before shutting down the library (and thread pool)
            let _ = player_shutdown_rx.recv();
            (LIBRARY_TX.get().expect(EXP_INIT))
                .send(LibraryRequest::Shutdown(library_shutdown_tx))
                .expect(EXP_RX);
        });

        imp.artists_page.uninit();
        imp.albums_page.uninit();
        imp.songs_page.uninit();
        imp.queue_page.uninit();

        let settings = self.settings();
        settings.set_int("window-width", self.size(Orientation::Horizontal))?;
        settings.set_int("window-height", self.size(Orientation::Vertical))?;
        settings.set_double("volume", settings_page.volume())?;
        settings.set_boolean("gapless", settings_page.gapless())?;
        settings.set_enum("startup-queue", *settings_page.startup_queue() as i32)?;
        settings.set_boolean("remember-time", remember_time)?;
        settings.set_boolean("adaptive-colors", settings_page.adaptive_colors())?;
        settings.set_enum("color-scheme", settings_page.color_scheme().cast_signed())?;
        settings.set_string("directories", &serialize_list(&settings_page.directories()))?;

        settings.set_boolean("songs-shuffle", imp.songs_page.get_shuffle())?;
        settings.set_boolean("albums-shuffle", imp.albums_page.get_shuffle())?;
        settings.set_boolean("artists-shuffle", imp.artists_page.get_shuffle())?;

        // Wait for all background tasks to complete before closing
        library_shutdown_rx.recv_timeout(Duration::from_millis(1500))?;
        Ok(())
    }

    pub fn load_state(&self) {
        let imp = self.imp();
        let settings_page = &imp.settings_page;
        let settings = self.settings();

        // Slider callback `change_value` doesn't work for `set_value()`,
        // so the volume has to be set manually before setting the slider
        let volume = settings.double("volume");
        settings_page
            .imp()
            .handle_set_volume(gtk::ScrollType::Jump, volume);
        settings_page.set_volume(volume);
        settings_page.set_gapless(settings.boolean("gapless"));
        settings_page.set_startup_queue(settings.enum_("startup-queue").into());
        settings_page.set_remember_time(settings.boolean("remember-time"));
        settings_page.set_adaptive_colors(settings.boolean("adaptive-colors"));
        settings_page.set_color_scheme(settings.enum_("color-scheme").cast_unsigned());

        imp.songs_page
            .set_shuffle(settings.boolean("songs-shuffle"));
        imp.albums_page
            .set_shuffle(settings.boolean("albums-shuffle"));
        imp.artists_page
            .set_shuffle(settings.boolean("artists-shuffle"));

        self.set_default_size(settings.int("window-width"), settings.int("window-height"));
    }
}
