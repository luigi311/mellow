use adw::Application;
use adw::{prelude::*, subclass::prelude::*};
use gdk::{DragAction, FileList};
use gio::Settings;
use glib::{Object, clone};
use gtk::{Orientation, gdk, gio, glib};
use std::sync::mpsc;
use std::time::Duration;

use crate::about;
use crate::excuses::{EXP_INIT, EXP_RX, INIT_ERR};
use crate::library::{LIBRARY_TX, LibraryRequest};
use crate::player::song_queue::SongQueue;
use crate::serializer::serialize_list;
use crate::{MUSIC_DIR, unescaped_split};

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
    pub fn new(app: &Application) -> Self {
        let window: Self = Object::builder().property("application", app).build();
        let imp = window.imp();
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
        window.load_state();
        window
    }

    fn setup_settings(&self) {
        let settings = Settings::new(about::app_id());
        self.imp().settings.set(settings).expect(INIT_ERR);
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
                    self.imp().library_songs_page.imp(),
                    move |_, _, _| songs_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_all_songs")
                .activate(clone!(
                    #[weak(rename_to=songs_page)]
                    self.imp().library_songs_page.imp(),
                    move |_, _, _| songs_page.handle_play_shuffled()
                ))
                .build(),
            gio::ActionEntry::builder("play_visible_album")
                .activate(clone!(
                    #[weak(rename_to=album_page)]
                    self.imp().library_album_page.imp(),
                    move |_, _, _| album_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_visible_album")
                .activate(clone!(
                    #[weak(rename_to=album_page)]
                    self.imp().library_album_page.imp(),
                    move |_, _, _| album_page.handle_play_shuffled()
                ))
                .build(),
            gio::ActionEntry::builder("play_visible_artist")
                .activate(clone!(
                    #[weak(rename_to=artist_page)]
                    self.imp().library_artist_page.imp(),
                    move |_, _, _| artist_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_visible_artist")
                .activate(clone!(
                    #[weak(rename_to=artist_page)]
                    self.imp().library_artist_page.imp(),
                    move |_, _, _| artist_page.handle_play_shuffled()
                ))
                .build(),
            gio::ActionEntry::builder("play_all_albums")
                .activate(clone!(
                    #[weak(rename_to=albums_page)]
                    self.imp().library_albums_page.imp(),
                    move |_, _, _| albums_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_all_albums")
                .activate(clone!(
                    #[weak(rename_to=albums_page)]
                    self.imp().library_albums_page.imp(),
                    move |_, _, _| albums_page.handle_play_shuffled()
                ))
                .build(),
            gio::ActionEntry::builder("play_all_artists")
                .activate(clone!(
                    #[weak(rename_to=artists_page)]
                    self.imp().library_artists_page.imp(),
                    move |_, _, _| artists_page.handle_play_sequential()
                ))
                .build(),
            gio::ActionEntry::builder("shuffle_all_artists")
                .activate(clone!(
                    #[weak(rename_to=artists_page)]
                    self.imp().library_artists_page.imp(),
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
            gio::ActionEntry::builder("library_nav_push")
                .parameter_type(Some(&String::static_variant_type()))
                .activate(clone!(
                    #[weak(rename_to=ui)]
                    self.imp(),
                    move |_, _, tag| {
                        let tag = tag.unwrap().get::<String>().unwrap();
                        ui.library_navigation_view.push_by_tag(&tag);
                    }
                ))
                .build(),
            gio::ActionEntry::builder("library_nav_pop")
                .activate(clone!(
                    #[weak(rename_to=ui)]
                    self.imp(),
                    move |_, _, _| {
                        ui.library_navigation_view.pop();
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
    pub fn save_state(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();
        let width = self.size(Orientation::Horizontal);
        let height = self.size(Orientation::Vertical);
        let settings_page = &imp.settings_page;
        let volume = settings_page.volume();
        let gapless = settings_page.gapless();
        let remember_queue = settings_page.remembers_queue();
        let directories = settings_page.directories();

        self.settings().set_int("window-width", width)?;
        self.settings().set_int("window-height", height)?;
        self.settings().set_double("volume", volume)?;
        self.settings().set_boolean("gapless", gapless)?;
        self.settings()
            .set_boolean("remember-queue", remember_queue)?;
        self.settings()
            .set_string("directories", &serialize_list(&directories))?;

        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let remember = imp.settings_page.remembers_queue();
        let song_queue = imp.song_queue.take();
        let playing_index = imp.song_queue_index.get();
        library_tx
            .send(LibraryRequest::RunTask(Box::new(move || {
                SongQueue::save_queue(remember, &song_queue, playing_index);
            })))
            .expect(EXP_RX);

        let (tx, rx) = mpsc::channel();
        library_tx.send(LibraryRequest::Shutdown(tx)).expect(EXP_RX);
        let _ = rx.recv_timeout(Duration::from_millis(1500));

        Ok(())
    }

    pub fn load_state(&self) {
        let settings = self.settings();
        let mut directories = unescaped_split(&settings.string("directories"), ',');
        if directories.is_empty() {
            directories.push(MUSIC_DIR.get().unwrap().clone());
        }
        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        library_tx
            .send(LibraryRequest::SetLibraries(directories.into()))
            .expect(EXP_RX);
        library_tx.send(LibraryRequest::Rebuild).expect(EXP_RX);
        library_tx.send(LibraryRequest::InitQueue).expect(EXP_RX);

        let settings_page = &self.imp().settings_page;
        let volume = settings.double("volume");
        let gapless = settings.boolean("gapless");
        let remember_queue = settings.boolean("remember-queue");

        // Slider callback `change_value` doesn't work for `set_value()`,
        // so the volume has to be manually updated before the slider
        settings_page
            .imp()
            .handle_set_volume(gtk::ScrollType::Jump, volume);

        settings_page.set_volume(volume);
        settings_page.set_gapless(gapless);
        settings_page.set_remember_queue(remember_queue);

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        self.set_default_size(width, height);
    }
}
