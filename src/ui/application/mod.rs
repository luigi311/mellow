use adw::prelude::*;
use gtk::{gio, glib};
use std::thread;

mod imp;

use crate::excuses::{EXP_INIT, EXP_RX, INIT_ERR};
use crate::library::{LIBRARY_TX, Library, LibraryConfig, LibraryRequest};
use crate::player::Player;
use crate::ui;
use crate::{MUSIC_DIR, about, unescaped_split};

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, adw::Application, gtk::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Application {
    pub fn new() -> Self {
        let app: Self = glib::Object::builder()
            .property("application-id", about::app_id())
            .property("flags", gio::ApplicationFlags::HANDLES_OPEN)
            .build();
        app.connect_open(|_, files, _| {
            let files = files
                .iter()
                .map(|file| file.path().unwrap().to_str().unwrap().to_owned())
                .collect();
            (LIBRARY_TX.get().expect(EXP_INIT))
                .send(LibraryRequest::QueueFromPaths(files))
                .expect(EXP_RX);
        });
        app.connect_startup(Self::init);
        app
    }

    #[inline]
    fn init(app: &Application) {
        let (mut player, player_tx, ui_tx, ui_rx) = Player::init();
        thread::Builder::new()
            .name("player".to_owned())
            .spawn(move || player.controller().unwrap())
            .expect(INIT_ERR);

        let settings = gio::Settings::new(about::app_id());
        let startup_queue = settings.enum_("startup-queue");
        let mut library = Library::init(
            LibraryConfig::new(match &*settings.string("directories") {
                // The value ":" means "first launch"
                ":" => vec![MUSIC_DIR.get().unwrap().clone()],
                dirs => unescaped_split(dirs, ','),
            }),
            player_tx,
            ui_tx,
        );
        thread::Builder::new()
            .name("library".to_owned())
            .spawn(move || {
                library.discover_files();
                library.init_queue(startup_queue).unwrap();
                library.request_handler().unwrap();
            })
            .expect(INIT_ERR);

        ui::init(app, settings, ui_rx);
    }
}
