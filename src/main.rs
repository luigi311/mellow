use adw::Application;
use adw::prelude::*;
use core::error::Error;
use gtk::gio;
use gtk::glib;
use std::sync::mpsc;
use std::thread;

use mellow::excuses::EXP_RX;
use mellow::library::{Library, LibraryRequest};
use mellow::player::{Player, PlayerRequest};
use mellow::{APP_ID, APP_NAME};

use mellow::excuses::INIT_ERR;

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
    let (mut player, player_tx, ui_tx, ui_rx) = Player::init().expect(INIT_ERR);
    let (mut library, library_tx) = Library::init(player_tx.clone(), ui_tx.clone());

    mellow::ui::init(app, &library_tx, &player_tx, ui_rx);

    thread::Builder::new()
        .name("player".to_string())
        .spawn(move || player.controller().unwrap())
        .unwrap();
    thread::Builder::new()
        .name("library".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Runtime::new().expect(INIT_ERR);
            runtime.block_on(async move {
                library_tx.send(LibraryRequest::Rebuild).expect(EXP_RX);
                init_player_queue(&library_tx, &player_tx).expect(INIT_ERR);
                library.request_handler().await.unwrap();
            });
        })
        .unwrap();
}

fn init_player_queue(
    library_tx: &mpsc::SyncSender<LibraryRequest>,
    player_tx: &mpsc::SyncSender<PlayerRequest>,
) -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args();
    args.next();
    if let Some(queue) = Library::queue_from_paths(&mut args) {
        player_tx.send(PlayerRequest::LoadQueue(queue))?;
        return Ok(());
    }

    // TODO: Instead of loading all tracks into the queue, either restore
    // the previous session or open the library without loading a queue
    // The library will have to be implemented first
    player_tx.send(PlayerRequest::SetShuffle(true))?;
    library_tx.send(LibraryRequest::QueueAllSongs)?;

    Ok(())
}
