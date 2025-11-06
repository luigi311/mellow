use adw::Application;
use adw::prelude::*;
use core::error::Error;
use gtk::gio;
use gtk::glib;
use mellow::player::PlayerRequest;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use tokio::sync::mpsc as tokio_mpsc;

use mellow::library::{Library, Song};
use mellow::player::Player;
use mellow::player::song_queue::QueueItem;
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

fn init(app: &Application) {
    let (mut player, player_tx, ui_tx, ui_rx) =
        Player::init().expect("Failed to initialize player");

    mellow::ui::build(app, &player_tx, ui_rx);

    #[allow(unused_must_use)]
    thread::Builder::new()
        .name("player".to_string())
        .spawn(move || {
            player.controller().inspect_err(|e| panic!("{e}"));
        });
    #[allow(unused_must_use)]
    thread::Builder::new()
        .name("init_player_queue".to_string())
        .spawn(move || {
            init_player_queue(player_tx, ui_tx).expect("Could not initialize player queue");
        });
}

fn init_player_queue(
    player_tx: mpsc::SyncSender<PlayerRequest>,
    ui_tx: tokio_mpsc::Sender<UpdateUI>,
) -> Result<(), Box<dyn Error>> {
    if let Some(queue) = queue_from_args() {
        player_tx.send(PlayerRequest::LoadQueue(queue))?;
        return Ok(());
    }

    let mut library = Library::load_or_init(ui_tx)?;
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
    let queue = library
        .songs
        .iter()
        .map(|song| QueueItem::Song(Arc::new(Mutex::new(song.clone()))))
        .collect();
    player_tx.send(PlayerRequest::SetShuffle(true))?;
    player_tx.send(PlayerRequest::LoadQueue(queue))?;

    Ok(())
}

fn queue_from_args() -> Option<Vec<QueueItem>> {
    let mut args = std::env::args();
    args.next();

    if args.len() == 0 {
        return None;
    }

    let queue = Arc::new(Mutex::new(Some(Vec::new())));
    args.for_each(|file| {
        let path = Path::new(&file);
        if path.is_file() {
            // Add files from arguments to queue
            if let Ok(song) = Song::new(&file, None) {
                if !Library::file_supported(&file) {
                    return;
                }
                let song = QueueItem::Song(Arc::new(Mutex::new(song)));
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
                    let song = QueueItem::Song(Arc::new(Mutex::new(song)));
                    queue.lock().unwrap().as_mut().unwrap().push(song);
                }
            });
        }
    });

    Some(queue.lock().unwrap().take().unwrap())
}
