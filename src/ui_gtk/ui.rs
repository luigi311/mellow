use adw::{self, Application};
use gst::State;
use gtk::gdk::Paintable;
use gtk::prelude::*;
use gtk::{Align, ApplicationWindow, Button, Orientation};
use std::sync::mpsc;

use crate::player::{PlayerRequest, PlayerResponse};

// TODO: Read the `gtk_rs` ebook and rewrite the UI in a more proper way

// TODO: When queue is empty, display a landing page
pub fn build(
    app: &Application,
    player_tx: &mpsc::SyncSender<PlayerRequest>,
    ui_rx: mpsc::Receiver<PlayerResponse>,
) {
    let main_view = gtk::Box::builder()
        .margin_top(4)
        .margin_bottom(12)
        .margin_end(26)
        .margin_start(26)
        .hexpand(true)
        .vexpand(true)
        .halign(Align::Center)
        .valign(Align::Center)
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();

    // TODO: Display the currently playing song album cover
    let cover = gtk::Picture::builder()
        .paintable(&Paintable::new_empty(1, 1))
        .content_fit(gtk::ContentFit::Contain)
        .halign(Align::Center)
        .height_request(185)
        .width_request(185)
        .css_classes(["card"])
        .build();
    main_view.append(&cover);

    // TODO: Display currently playing song/album/atrist
    // TODO: Marquee long titles
    let title_label = gtk::Label::builder()
        .label("<b>Song Title</b>")
        .margin_top(6)
        .use_markup(true)
        .build();
    let album_label = gtk::Label::builder().label("Album Title").build();
    let artist_label = gtk::Label::builder()
        .label("Band Name")
        .margin_bottom(6)
        .build();
    main_view.append(&title_label);
    main_view.append(&album_label);
    main_view.append(&artist_label);

    // TODO: Overlay media controls & auto-hide
    let media_toolbar = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .hexpand(true)
        .css_classes(["toolbar", "osd"])
        .build();

    let player_controls = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .halign(Align::Center)
        .hexpand(true)
        .margin_start(6)
        .margin_end(6)
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
    player_controls.append(&prev_button);

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
    player_controls.append(&pause_button);

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
    player_controls.append(&next_button);

    let seek_controls = gtk::Box::builder().hexpand(true).build();

    // TODO: Responsive seek bar/slider
    // TODO: Seek bar time labels
    let seek_bar = gtk::Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
    seek_bar.set_hexpand(true);
    seek_bar.set_margin_start(6);
    seek_bar.set_margin_end(6);
    seek_bar.connect_value_changed(move |_scale| {
        // TODO: Seek usink the seek bar
        // println!("{}", scale.value());
    });

    let time_cur_label = gtk::Label::builder()
        .label("-:--")
        .halign(Align::Start)
        .build();
    let time_end_label = gtk::Label::builder()
        .label("-:--")
        .halign(Align::End)
        .build();
    seek_controls.append(&time_cur_label);
    seek_controls.append(&seek_bar);
    seek_controls.append(&time_end_label);

    media_toolbar.append(&player_controls);
    media_toolbar.append(&seek_controls);

    main_view.append(&media_toolbar);

    let titlebar = adw::HeaderBar::builder()
        .show_title(false)
        .css_classes(["flat"])
        .build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Mellow")
        .titlebar(&titlebar)
        .child(&main_view)
        .build();
    window.present();
}
