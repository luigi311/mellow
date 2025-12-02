use adw::Application;
use adw::prelude::*;
use core::error::Error;
use gtk::gio;
use gtk::glib;
use std::sync::mpsc;
use std::thread;
use tokio::sync::mpsc as tokio_mpsc;

use mellow::library::Library;
use mellow::player::Player;
use mellow::player::PlayerRequest;
use mellow::ui::UpdateUI;
use mellow::{APP_ID, APP_NAME};

pub fn main() -> gtk::glib::ExitCode {
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
    let (mut player, player_tx, ui_tx, ui_rx) = Player::init().unwrap();

    mellow::ui::init(app, &player_tx, ui_rx);

    thread::Builder::new()
        .name("player".to_string())
        .spawn(move || player.controller().unwrap())
        .unwrap();
    thread::Builder::new()
        .name("init_player_queue".to_string())
        .spawn(move || init_player_queue(player_tx, ui_tx).unwrap())
        .unwrap();
}

fn init_player_queue(
    player_tx: mpsc::SyncSender<PlayerRequest>,
    ui_tx: tokio_mpsc::Sender<UpdateUI>,
) -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args();
    args.next();
    if let Some(queue) = Library::queue_from_paths(&mut args) {
        player_tx.send(PlayerRequest::LoadQueue(queue))?;
        return Ok(());
    }

    let mut library = Library::load_or_init(ui_tx);
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| e.to_string())
        .unwrap();
    let library = runtime.block_on(async move {
        library.rebuild().await.unwrap();
        library
    });

    // TODO: Instead of loading all tracks into the queue, either restore
    // the previous session or open the library without loading a queue
    // The library will have to be implemented first
    player_tx.send(PlayerRequest::SetShuffle(true))?;
    player_tx.send(PlayerRequest::LoadQueue(library.queue_all_songs()))?;

    Ok(())
}
