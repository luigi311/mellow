use adw::{self, Application, ApplicationWindow};
use glib::clone;
use gst::State;
use gtk::pango::EllipsizeMode;
use gtk::{self, Align, Orientation, gdk, glib, prelude::*};
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::format_duration;
use crate::player::{PlayerRequest, PlayerResponse};
use crate::{APP_ID, APP_NAME};

// TODO: Use `.ui` files for building the interface
// TODO: Implement UI changes from the `relm4` branch

// TODO: When queue is empty, display a landing page
pub fn build(
    app: &Application,
    player_tx: &mpsc::SyncSender<PlayerRequest>,
    mut ui_rx: tokio_mpsc::Receiver<PlayerResponse>,
) {
    let player_view = gtk::Box::builder()
        .margin_top(0)
        .margin_bottom(12)
        .margin_end(26)
        .margin_start(26)
        .hexpand(true)
        .vexpand(true)
        .valign(Align::Center)
        .orientation(Orientation::Vertical)
        .spacing(6)
        .build();
    let window_handle = gtk::WindowHandle::builder().child(&player_view).build();
    let player_ui = adw::ToolbarView::builder().content(&window_handle).build();
    player_ui.add_top_bar(
        &adw::HeaderBar::builder()
            .show_title(false)
            .css_classes(["flat"])
            .build(),
    );

    // TODO: Display the currently playing song album cover
    let album_cover = gtk::Picture::builder()
        .paintable(&gdk::Paintable::new_empty(1, 1))
        .content_fit(gtk::ContentFit::Contain)
        .halign(Align::Center)
        .height_request(0)
        .width_request(0)
        .css_classes(["card"])
        .build();
    player_view.append(&album_cover);

    // TODO: Marquee long titles
    let title_label = gtk::Label::builder()
        .css_classes(["heading"])
        .ellipsize(EllipsizeMode::End)
        .margin_top(6)
        .build();
    let album_label = gtk::Label::builder()
        .css_classes(["caption-heading"])
        .ellipsize(EllipsizeMode::End)
        .build();
    let artist_label = gtk::Label::builder()
        .css_classes(["caption-heading"])
        .ellipsize(EllipsizeMode::End)
        .margin_bottom(6)
        .build();
    player_view.append(&title_label);
    player_view.append(&album_label);
    player_view.append(&artist_label);

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

    let prev_button = gtk::Button::builder()
        .css_classes(["circular"])
        .icon_name("media-skip-backward-symbolic")
        .build();
    prev_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| player_tx.send(PlayerRequest::SkipPrevious).unwrap()
    });
    let pause_button = gtk::Button::builder()
        .icon_name("media-playback-start-symbolic")
        .css_classes(["circular"])
        .build();
    pause_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| player_tx.send(PlayerRequest::PlayOrPause).unwrap()
    });
    let next_button = gtk::Button::builder()
        .css_classes(["circular"])
        .icon_name("media-skip-forward-symbolic")
        .build();
    next_button.connect_clicked({
        let player_tx = player_tx.clone();
        move |_| player_tx.send(PlayerRequest::SkipNext).unwrap()
    });

    player_controls.append(&prev_button);
    player_controls.append(&pause_button);
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

    player_view.append(&media_toolbar);

    // TODO: Library interface
    let bottom_bar = gtk::Box::builder().height_request(30).build();
    bottom_bar.append(
        &gtk::Image::builder()
            .icon_name("view-continuous-symbolic")
            .css_classes(["dimmed"])
            .halign(Align::Center)
            .hexpand(true)
            .build(),
    );
    player_view.append(&gtk::Box::builder().height_request(15).build());
    let bottom_sheet = adw::ToolbarView::builder().build();
    let library_ui = adw::ToolbarView::builder().vexpand(true).build();
    let library_view = adw::StatusPage::builder().build();
    let library_content = gtk::Box::builder().height_request(9999).build();
    bottom_sheet.add_top_bar(&adw::HeaderBar::new());
    library_view.set_child(Some(&library_content));
    library_ui.set_content(Some(&library_view));
    bottom_sheet.set_content(Some(&library_ui));
    let player_and_library_ui = adw::BottomSheet::builder()
        .content(&player_ui)
        .bottom_bar(&bottom_bar)
        .sheet(&bottom_sheet)
        .build();

    ApplicationWindow::builder()
        .content(&player_and_library_ui)
        // .content(&player_ui)
        .default_width(270)
        .default_height(450)
        .width_request(0)
        .height_request(0)
        .title(APP_NAME)
        .name(APP_NAME)
        .icon_name(APP_ID)
        .application(app)
        .build()
        .present();

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

                        if let Some(artwork) = song_info.artwork.as_ref() {
                            album_cover.set_paintable(Some(artwork));
                        } else {
                            album_cover.set_paintable(Some(&gdk::Paintable::new_empty(1, 1)));
                        }
                        album_cover.set_height_request(0);
                        album_cover.set_width_request(0);
                        title_label.set_label(&song_info.title);
                        album_label.set_label(&song_info.album);
                        artist_label.set_label(&song_info.artist);

                        time_end_label.set_label(&format_duration(&Duration::from_millis(
                            song_info.duration.mseconds(),
                        )));
                    }
                    PlayerResponse::Time(time) => {
                        time_cur_label.set_label(&format_duration(&Duration::from_millis(
                            time.map_or_else(|| 0, gst::ClockTime::mseconds),
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
