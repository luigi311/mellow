use adw::Application;
use gtk::{glib, prelude::*};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use mellow::library::{Library, Song};
use mellow::player::Player;
use mellow::visit_dirs;
use mellow::{APP_ID, APP_NAME};

pub fn main() -> gtk::glib::ExitCode {
    glib::set_application_name(APP_NAME);
    glib::set_program_name(Some(APP_NAME.to_lowercase()));

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(init);
    app.run_with_args(&[] as &[&str; 0])
}

// NOTE: It might be a good idea to rewrite all of the UI
// from scratch once done experimenting

fn init(app: &Application) {
    let (mut player, player_tx, ui_rx) = Player::init().expect("Failed to initialize player");

    mellow::ui::build(app, &player_tx, ui_rx);

    thread::Builder::new()
        .name("player".to_string())
        .spawn(move || {
            init_player_queue(&mut player);
            player_tx.send(mellow::PlayerRequest::Update).unwrap();
            player
                .event_handler(player_tx)
                .expect("Player thread crashed")
        })
        .unwrap();
}

fn init_player_queue(player: &mut Player) {
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
                };
            } else if Path::exists(path) {
                // Add all files within directory arguments to queue
                let _ = visit_dirs(path, &|file| {
                    let file = file.path();
                    let file = file.to_str().unwrap();
                    if let Ok(song) = Song::new(file, None) {
                        if !Library::file_supported(&file) {
                            return;
                        }
                        queue.lock().unwrap().as_mut().unwrap().push(song);
                    };
                });
            }
        });
        player.new_queue(queue.lock().unwrap().take().unwrap());
    } else {
        let mut library = Library::load_or_init().expect("Library could not be initialized");
        library.rebuild().unwrap();
        player.shuffle = true;
        player.new_queue(library.songs);
        player.randomize_queue();
    }
}
