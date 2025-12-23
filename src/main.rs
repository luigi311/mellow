use adw::{Application, prelude::*};
use gtk::{gio, glib};
use std::thread;

use mellow::excuses::INIT_ERR;
use mellow::library::Library;
use mellow::player::Player;
use mellow::{APP_ID, APP_NAME};

pub fn main() -> glib::ExitCode {
    glib::set_application_name(APP_NAME);
    glib::set_program_name(Some(APP_NAME.to_lowercase()));

    gio::resources_register_include!("mellow.gresource").expect("Failed to register resources");

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(init);
    app.set_accels_for_action("window.close", &["<Ctrl>W", "<Ctrl>Q"]);
    app.set_accels_for_action("player.play_pause", &["space"]);
    app.run_with_args(&[] as &[&str; 0])
}

fn init(app: &Application) {
    mellow::init_globals().expect(INIT_ERR);
    let (mut player, player_tx, ui_tx, ui_rx) = Player::init().expect(INIT_ERR);
    let mut library = Library::init(player_tx, ui_tx.clone());

    mellow::ui::init(app, &ui_tx, ui_rx);

    thread::Builder::new()
        .name("library".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect(INIT_ERR);
            runtime.block_on(async move { library.request_handler().await.unwrap() });
        })
        .expect(INIT_ERR);
    thread::Builder::new()
        .name("player".to_string())
        .spawn(move || player.controller().unwrap())
        .expect(INIT_ERR);
}
