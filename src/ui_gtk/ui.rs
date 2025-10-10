use adw::{self, Application};
use gst::State;
use gtk::gdk::Paintable;
use gtk::glib::{self, clone};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::{Align, ApplicationWindow, Button, Orientation};
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::format_duration;
use crate::player::{PlayerRequest, PlayerResponse};

// TODO: Use `.ui` files for building the interface
// TODO: Implement UI changes from the `relm4` branch

// TODO: When queue is empty, display a landing page
pub fn build(
    app: &Application,
    player_tx: &mpsc::SyncSender<PlayerRequest>,
    mut ui_rx: tokio_mpsc::Receiver<PlayerResponse>,
) {
    let main_view = gtk::Box::builder()
        .margin_top(4)
        .margin_bottom(12)
        .margin_end(26)
        .margin_start(26)
        .hexpand(true)
        .vexpand(true)
        .valign(Align::Center)
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();

    // TODO: Display the currently playing song album cover
    let album_cover = gtk::Picture::builder()
        .paintable(&Paintable::new_empty(1, 1))
        .content_fit(gtk::ContentFit::Contain)
        .halign(Align::Center)
        .height_request(185)
        .width_request(185)
        .css_classes(["card"])
        .build();
    main_view.append(&album_cover);

    // TODO: Marquee long titles
    let title_label = gtk::Label::builder()
        .label("Song Title")
        .css_classes(["heading"])
        .ellipsize(EllipsizeMode::End)
        .margin_top(6)
        .build();
    let album_label = gtk::Label::builder()
        .label("Album Title")
        .css_classes(["caption-heading"])
        .ellipsize(EllipsizeMode::End)
        .build();
    let artist_label = gtk::Label::builder()
        .label("Band Name")
        .css_classes(["caption-heading"])
        .ellipsize(EllipsizeMode::End)
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

    let prev_button = Button::builder()
        .css_classes(["circular"])
        .icon_name("media-skip-backward-symbolic")
        .build();
    prev_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| player_tx.send(PlayerRequest::SkipPrevious).unwrap()
    });
    player_controls.append(&prev_button);

    let pause_button = Button::builder()
        .icon_name("media-playback-start-symbolic")
        .css_classes(["circular"])
        .build();
    pause_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| player_tx.send(PlayerRequest::PlayOrPause).unwrap()
    });
    player_controls.append(&pause_button);

    let next_button = Button::builder()
        .css_classes(["circular"])
        .icon_name("media-skip-forward-symbolic")
        .build();
    next_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| player_tx.send(PlayerRequest::SkipNext).unwrap()
    });
    player_controls.append(&next_button);

    let seek_controls = gtk::Box::builder().hexpand(true).build();

    let seek_bar = gtk::Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
    seek_bar.set_hexpand(true);
    seek_bar.set_margin_start(6);
    seek_bar.set_margin_end(6);
    seek_bar.connect_value_changed({
        let player_tx = player_tx.clone();
        move |scale| player_tx.send(PlayerRequest::Seek(scale.value())).unwrap()
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

    glib::spawn_future_local(clone!(
        #[weak]
        album_cover,
        #[weak]
        title_label,
        #[weak]
        album_label,
        #[weak]
        artist_label,
        #[weak]
        pause_button,
        #[weak]
        time_cur_label,
        #[weak]
        seek_bar,
        #[weak]
        time_end_label,
        async move {
            loop {
                let Some(response) = ui_rx.recv().await else {
                    continue;
                };

                match response {
                    // TODO: Disable buttons based on state (loading library, no queue, etc)
                    PlayerResponse::State(state) => {
                        pause_button.set_icon_name(match state {
                            State::Playing => "media-playback-pause-symbolic",
                            _ => "media-playback-start-symbolic",
                        });
                    }
                    PlayerResponse::SongInfo(song_info) => {
                        let Some(song_info) = song_info else { return };

                        album_cover.set_paintable(song_info.artwork.as_ref());
                        title_label.set_label(&song_info.title);
                        album_label.set_label(&song_info.album);
                        artist_label.set_label(&song_info.artist);

                        time_end_label.set_label(&format_duration(&Duration::from_millis(
                            song_info.duration.mseconds(),
                        )));
                    }
                    PlayerResponse::Time(time) => {
                        time_cur_label.set_label(&format_duration(&Duration::from_millis(
                            time.map_or_else(|| 5000, |time| time.mseconds()),
                        )));
                        // TODO: Grey-out the slider when no song is active
                        // TODO: Update the seek bar/slider and labels to show the correct time
                        // It might be better to either use the range as (milli)seconds
                        // or the `time` to become a ratio value, so it's easier to set
                        // the fill level
                        // seek_bar.set_fill_level();
                    }
                }
            }
        },
    ));
}
