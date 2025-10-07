use adw::{self, Application};
use gst::State;
use gtk::prelude::*;
use gtk::{Align, ApplicationWindow, Button, Orientation};
use std::sync::mpsc;

use crate::player::{PlayerRequest, PlayerResponse};

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
        .spacing(6)
        .build();

    // TODO: Display the album cover

    // TODO: Display currently playing song/album/atrist
    // TODO: Marquee long titles
    // let title_label = gtk::Label::builder().label("Song Title").build();
    // let album_label = gtk::Label::builder().label("Album Title").build();
    // let artist_label = gtk::Label::builder().label("Band Name").build();
    // gtk_box_main.append(&title_label);
    // gtk_box_main.append(&album_label);
    // gtk_box_main.append(&artist_label);

    // TODO: Overlay media controls & auto-hide
    let gtk_box_media_toolbar = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(["toolbar", "osd"])
        .build();

    let gtk_box_player_controls = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();

    // TODO: Disable buttons based on state (loading library, no queue, etc)
    let prev_button = Button::builder()
        .css_classes(["circular"])
        .icon_name("media-skip-backward-symbolic")
        .build();
    prev_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| {
            player_tx.send(PlayerRequest::SkipPrevious).unwrap();
        }
    });
    gtk_box_player_controls.append(&prev_button);

    // TODO: Change symbol if stopped (like when the queue ends)
    let pause_button = Button::builder()
        .icon_name("media-playback-start-symbolic")
        .css_classes(["circular"])
        .build();
    pause_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |button| {
            player_tx.send(PlayerRequest::PlayOrPause).unwrap();
            player_tx.send(PlayerRequest::GetCurrentState).unwrap();
            if let PlayerResponse::State(state) = ui_rx.recv().unwrap() {
                button.set_icon_name(match state {
                    State::Playing => "media-playback-pause-symbolic",
                    _ => "media-playback-start-symbolic",
                });
            }
        }
    });
    gtk_box_player_controls.append(&pause_button);

    let next_button = Button::builder()
        .css_classes(["circular"])
        .icon_name("media-skip-forward-symbolic")
        .build();
    next_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| {
            player_tx.send(PlayerRequest::SkipNext).unwrap();
        }
    });
    gtk_box_player_controls.append(&next_button);

    // TODO: Responsive seek bar/slider
    // TODO: Seek bar time labels
    // let seek_bar = gtk::Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.001);
    // seek_bar.connect_value_changed(move |scale| {
    //     println!("{}", scale.value());
    // });

    gtk_box_media_toolbar.append(&gtk_box_player_controls);
    // gtk_box_media_toolbar.append(&seek_bar);

    gtk_box_main.append(&gtk_box_media_toolbar);

    let titlebar = adw::HeaderBar::builder()
        .show_title(false)
        .css_classes(["flat"])
        .build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Mellow")
        .titlebar(&titlebar)
        .child(&gtk_box_main)
        .build();
    window.present();
}
