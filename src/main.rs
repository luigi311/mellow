use adw::Application;
use core::error::Error;
use gtk::prelude::*;
use std::thread;

const APP_ID: &str = "com.github.userwithaname.Mellow";

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

    thread::Builder::new()
        .name("player".to_string())
        .spawn(move || {
            init_player_queue(&mut player);
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
        player.new_queue(
            args.filter_map(|file| Song::new(&file, None).ok())
                .collect(),
        );
    } else {
        let library = Library::rebuild().unwrap();
        player.shuffle = true;
        player.new_queue(library.songs);
        player.randomize_queue();
    }
}
