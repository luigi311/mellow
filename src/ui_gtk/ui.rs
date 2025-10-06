use adw::{self, Application};
use gst::State;
use gtk::prelude::*;
use gtk::{Align, ApplicationWindow, Button, Orientation};
use std::sync::mpsc;

use crate::player::{PlayerRequest, PlayerResponse};

// TODO: Display currently playing song/album/atrist
// TODO: Marquee long titles
// TODO: Display album cover
// TODO: When queue is empty, display a landing page
pub fn build(
    app: &Application,
    player_tx: &mpsc::SyncSender<PlayerRequest>,
    ui_rx: mpsc::Receiver<PlayerResponse>,
) {
    let gtk_box_main = gtk::Box::builder()
        .margin_top(12)
        .margin_bottom(12)
        .margin_end(12)
        .margin_start(12)
        .valign(Align::Center)
        .halign(Align::Center)
        .orientation(Orientation::Vertical)
        .spacing(12)
        .build();

    let gtk_box_player_controls = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();

    // TODO: Symbolic icons
    // TODO: Disable buttons based on state (loading library, no queue, etc)
    let prev_button = Button::builder().label("⏮").build();
    prev_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| {
            player_tx.send(PlayerRequest::SkipPrevious).unwrap();
        }
    });
    gtk_box_player_controls.append(&prev_button);

    // TODO: Change symbol if stopped (like when the queue ends)
    let pause_button = Button::builder().label("▶").build();
    pause_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |button| {
            player_tx.send(PlayerRequest::PlayOrPause).unwrap();
            player_tx.send(PlayerRequest::GetCurrentState).unwrap();
            if let PlayerResponse::State(state) = ui_rx.recv().unwrap() {
                button.set_label(match state {
                    State::Playing => "⏸",
                    _ => "▶",
                });
            }
        }
    });
    gtk_box_player_controls.append(&pause_button);

    let next_button = Button::builder().label("⏭").build();
    next_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| {
            player_tx.send(PlayerRequest::SkipNext).unwrap();
        }
    });
    gtk_box_player_controls.append(&next_button);

    gtk_box_main.append(&gtk_box_player_controls);

    // TODO: Seek bar/slider
    // let seek_bar = Scale::builder()
    //     .fill_level(0.0)
    //     .orientation(Orientation::Horizontal)
    //     .build();
    // seek_bar.connect_value_changed(move |scale| {
    //     println!("{}", scale.value());
    // });
    // gtk_box_main.append(&seek_bar);

    // let titlebar = gtk::Box::builder().build();
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Mellow")
        // .titlebar(&titlebar)
        .child(&gtk_box_main)
        .build();
    window.present();
}
