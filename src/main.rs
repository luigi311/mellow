use adw::Application;
use core::error::Error;
use gtk::prelude::*;
use std::thread;

const APP_ID: &str = "org.test.MusicPlayer";

use mellow::library::{Library, Song};
use mellow::player::Player;

pub fn main() -> Result<(), Box<dyn Error>> {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(init);

    app.run_with_args(&[] as &[&str; 0]);

    Ok(())
}

// NOTE: It might be a good idea to rewrite all of the UI
// from scratch once done experimenting

fn init(app: &Application) {
    let (mut player, player_tx, ui_rx) = Player::init().expect("Failed to initialize player");

    mellow::ui_gtk::build(app, &player_tx, ui_rx);

    let mut args = std::env::args();
    args.next();
    if args.len() > 0 {
        player
            .new_queue(
                args.filter_map(|file| Song::new(&file, None).ok())
                    .collect(),
            )
            .expect("Failed to create player queue");
    } else {
        // TODO: Don't block `app.connect_activate()` with long operations
        let library = Library::rebuild().unwrap();
        player.shuffle = true;
        player
            .new_queue(library.songs)
            .expect("Failed to create player queue");
    }

    thread::Builder::new()
        .name("player".to_string())
        .spawn(move || {
            player
                .event_handler(player_tx)
                .expect("Player thread crashed")
        })
        .unwrap();
}
