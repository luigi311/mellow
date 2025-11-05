use adw::Application;
use adw::prelude::*;
use gtk::gio;
use gtk::glib;
use std::path::Path;
use std::sync::{Arc, Mutex};
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
            init_player_queue(&mut player, ui_tx);
            player.controller().inspect_err(|e| panic!("{e}"));
        });
}

fn init_player_queue(player: &mut Player, ui_tx: tokio_mpsc::Sender<UpdateUI>) {
    if let Some(queue) = queue_from_args() {
        player.queue.load_new(queue, None, None).unwrap();
        return;
    }

    let mut library = Library::load_or_init(ui_tx).expect("Library could not be initialized");
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| e.to_string())
        .unwrap();
    let library = runtime.block_on(async move {
        library.rebuild().await.unwrap();
        library
    });

    // TODO: Once the library works, don't load all tracks into the queue,
    // but instead either restore the previous session or open the library
    // without loading a queue
    if player.queue.is_empty() {
        let songs = library
            .songs
            .iter()
            .map(|song| QueueItem::Song(Arc::new(Mutex::new(song.clone()))))
            .collect();
        player.queue.load_new(songs, Some(true), None).unwrap();
    }
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
