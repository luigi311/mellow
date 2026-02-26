use adw::prelude::*;
use gtk::{gio, glib};
use std::thread;

use mellow::excuses::{EXP_INIT, EXP_RX, INIT_ERR};
use mellow::library::config::LibraryConfig;
use mellow::library::{LIBRARY_TX, Library, LibraryRequest};
use mellow::player::Player;
use mellow::ui::application::Application;
use mellow::{MUSIC_DIR, about, unescaped_split};

pub fn main() -> glib::ExitCode {
    glib::set_application_name(about::app_name());
    glib::set_program_name(Some(about::app_name().to_lowercase()));

    register_resources();
    mellow::init_globals();

    let app = Application::new();
    app.connect_startup(init);
    app.connect_open(|_, files, _| {
        let files = files
            .iter()
            .map(|file| file.path().unwrap().to_str().unwrap().to_owned())
            .collect();
        (LIBRARY_TX.get().expect(EXP_INIT))
            .send(LibraryRequest::QueueFromPaths(files))
            .expect(EXP_RX);
    });

    app.set_accels_for_action("window.close", &["<Ctrl>W", "<Ctrl>Q"]);
    app.set_accels_for_action("win.queue_from_disk", &["<Ctrl>O"]);
    // TODO: Ignore shortcut when the overlay is open
    // app.set_accels_for_action("player.play_pause", &["space"]);

    app.run()
}

#[inline]
fn register_resources() {
    #[cfg(feature = "no-meson")]
    gio::resources_register_include!("mellow.gresource").expect("Failed to register resources");

    #[cfg(not(feature = "no-meson"))]
    gio::resources_register(
        &gio::Resource::load(mellow::about::resources_file())
            .expect("Could not load resources file"),
    );
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

    mellow::ui::init(app, settings, ui_rx);
}
