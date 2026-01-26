use adw::{Application, prelude::*};
use gtk::{gio, glib};
use std::thread;

use mellow::about;
use mellow::excuses::INIT_ERR;
use mellow::library::Library;
use mellow::player::Player;

pub fn main() -> glib::ExitCode {
    glib::set_application_name(about::app_name());
    glib::set_program_name(Some(about::app_name().to_lowercase()));

    gio::resources_register(
        &gio::Resource::load(mellow::about::resources_file())
            .expect("Could not load resources file"),
    );

    let app = Application::builder()
        .application_id(mellow::about::app_id())
        .build();
    app.connect_activate(init);
    app.set_accels_for_action("window.close", &["<Ctrl>W", "<Ctrl>Q"]);
    app.set_accels_for_action("player.play_pause", &["space"]);
    app.run_with_args(&[] as &[&str; 0])
}

#[inline]
fn init(app: &Application) {
    mellow::init_globals().expect(INIT_ERR);
    let (mut player, player_tx, ui_tx, ui_rx) = Player::init().expect(INIT_ERR);
    let mut library = Library::init(player_tx, ui_tx);

    thread::Builder::new()
        .name("library".to_string())
        .spawn(move || library.request_handler().unwrap())
        .expect(INIT_ERR);
    thread::Builder::new()
        .name("player".to_string())
        .spawn(move || player.controller().unwrap())
        .expect(INIT_ERR);

    mellow::ui::init(app, ui_rx);
}
