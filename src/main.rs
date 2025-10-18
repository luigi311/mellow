use adw::Application;
use adw::prelude::*;
use gtk::gio;
use gtk::glib;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc as tokio_mpsc;

use mellow::library::{Library, Song};
use mellow::player::{Player, PlayerRequest};
use mellow::ui::UpdateUI;
use mellow::visit_dirs;
use mellow::{APP_ID, APP_NAME};

pub fn main() -> gtk::glib::ExitCode {
    glib::set_application_name(APP_NAME);
    glib::set_program_name(Some(APP_NAME.to_lowercase()));

    gio::resources_register_include!("mellow.gresource").expect("Failed to register resources");

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(init);
    app.set_accels_for_action("window.close", &["<Ctrl>W", "<Ctrl>Q"]);
    app.run_with_args(&[] as &[&str; 0])
}

// NOTE: It might be a good idea to rewrite all of the UI
// from scratch once done experimenting

fn init(app: &Application) {
    let (mut player, player_tx, ui_tx, ui_rx) =
        Player::init().expect("Failed to initialize player");

    mellow::ui::build(app, &player_tx, ui_rx);

    #[allow(unused_must_use)]
    thread::Builder::new()
        .name("player".to_string())
        .spawn(move || {
            init_player_queue(&mut player, ui_tx);
            player_tx.send(PlayerRequest::Update).unwrap();
            player.controller().inspect_err(|e| panic!("{e}"));
        });
}

fn init_player_queue(player: &mut Player, ui_tx: tokio_mpsc::Sender<UpdateUI>) {
    let mut args = std::env::args();
    args.next();
    if args.len() > 0 {
        let queue = Arc::new(Mutex::new(Some(Vec::new())));
        args.for_each(|file| {
            let path = Path::new(&file);
            if path.is_file() {
                // Add files from arguments to queue
                if let Ok(song) = Song::new(&file, None) {
                    if !Library::file_supported(&file) {
                        return;
                    }
                    queue.lock().unwrap().as_mut().unwrap().push(song);
                }
            } else if Path::exists(path) {
                // Add all files within directory arguments to queue
                let _ = visit_dirs(path, &|file| {
                    let file = file.path();
                    let file = file.to_str().unwrap();
                    if let Ok(song) = Song::new(file, None) {
                        if !Library::file_supported(file) {
                            return;
                        }
                        queue.lock().unwrap().as_mut().unwrap().push(song);
                    }
                });
            }
        });
        player.new_queue(queue.lock().unwrap().take().unwrap());
    } else {
        let mut library = Library::load_or_init(ui_tx).expect("Library could not be initialized");
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| e.to_string())
            .unwrap();
        let songs = runtime.block_on(async move {
            library.rebuild().await.unwrap();
            library.songs
        });
        player.new_queue(songs);
    }
}
