use adw::{self, Application, prelude::*};
use glib::clone;
use gst::{ClockTime, State};
use gtk::pango::EllipsizeMode;
use gtk::{self, Align, Orientation, gdk, glib};
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::format_duration;
use crate::library::SongInfo;
use crate::player::PlayerRequest;
use crate::window::Window;
use crate::{APP_ID, APP_NAME};

pub enum UpdateUI {
    PlayerState(State),
    PlayerTime(Option<ClockTime>),
    SongInfo(Option<Box<SongInfo>>),
    Progress(Option<f64>),
}

// TODO: Use `.ui` files for building the interface
// TODO: Implement UI changes from the `relm4` branch

// TODO: When queue is empty, display a landing page
pub fn build(
    app: &Application,
    player_tx: &mpsc::SyncSender<PlayerRequest>,
    mut ui_rx: tokio_mpsc::Receiver<UpdateUI>,
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
    let progress_bar = gtk::ProgressBar::builder()
        .hexpand(true)
        .fraction(0.5)
        .visible(false)
        .build();
    progress_bar.add_css_class("osd");
    player_ui.add_top_bar(&progress_bar);

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
    seek_bar.connect_change_value({
        let player_tx = player_tx.clone();
        move |_, _, value| {
            player_tx.send(PlayerRequest::Seek(value)).unwrap();
            glib::Propagation::Proceed
        }
    });

    let time_cur_label = gtk::Label::builder()
        .css_classes(["numeric"])
        .halign(Align::Start)
        .label("-:--")
        .build();
    let time_end_label = gtk::Label::builder()
        .css_classes(["numeric"])
        .halign(Align::End)
        .label("-:--")
        .build();
    seek_controls.append(&time_cur_label);
    seek_controls.append(&seek_bar);
    seek_controls.append(&time_end_label);

    media_toolbar.append(&player_controls);
    media_toolbar.append(&seek_controls);

    player_view.append(&media_toolbar);

    // TODO: Library interface
    player_view.append(&gtk::Box::builder().height_request(15).build());
    let bottom_bar = gtk::Box::builder()
        .css_classes(["flat"])
        .height_request(30)
        .build();
    bottom_bar.append(
        &gtk::Image::builder()
            .icon_name("view-continuous-symbolic")
            .css_classes(["dimmed"])
            .halign(Align::Center)
            .hexpand(true)
            .build(),
    );

    // IDEA: Show a bottom bar when the sheet is open, to quickly close it
    // without having to reach to the top. The bar would be located below
    // the tabs (if it's not too much clutter), and display the currently
    // playing track info.
    let bottom_sheet = adw::ToolbarView::builder().build();
    let library_ui = adw::ToolbarView::builder()
        .vexpand(true)
        .hexpand(true)
        .build();
    let library_view = adw::StatusPage::builder().build();
    let library_content = gtk::Box::builder().vexpand(true).build();

    let lyrics_label = gtk::Label::builder()
        .hexpand(true)
        .halign(Align::Center)
        .justify(gtk::Justification::Center)
        .wrap_mode(gtk::pango::WrapMode::Word)
        .wrap(true)
        .margin_start(12)
        .margin_end(12)
        .css_classes(["body"])
        .build();
    // IDEA: Lyrics could be a tab in the bottom sheet
    library_content.append(&lyrics_label);
    bottom_sheet.add_top_bar(&adw::HeaderBar::new());
    library_view.set_child(Some(&library_content));
    library_ui.set_content(Some(&library_view));
    bottom_sheet.set_content(Some(&library_ui));
    let player_and_library_ui = adw::BottomSheet::builder()
        .content(&player_ui)
        .bottom_bar(&bottom_bar)
        .sheet(&bottom_sheet)
        .build();

    let window = Window::new(&app);
    window.set_content(Some(&player_and_library_ui));
    window.set_title(Some(APP_NAME));
    window.set_icon_name(Some(APP_ID));

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
        #[weak]
        lyrics_label,
        #[weak]
        progress_bar,
        async move {
            let mut song_duration = Duration::from_secs(0);
            loop {
                let Some(response) = ui_rx.recv().await else {
                    continue;
                };

                match response {
                    // TODO: Disable buttons based on state (loading library, no queue, etc)
                    UpdateUI::PlayerState(state) => {
                        pause_button.set_icon_name(match state {
                            State::Playing => "media-playback-pause-symbolic",
                            _ => "media-playback-start-symbolic",
                        });
                    }
                    UpdateUI::SongInfo(song_info) => {
                        let Some(song_info) = song_info else { return };

                        if let Some(artwork) = song_info.artwork.as_ref() {
                            album_cover.set_paintable(Some(artwork));
                        } else {
                            album_cover.set_paintable(Some(&gdk::Paintable::new_empty(1, 1)));
                        }
                        // IDEA: Once the controls toolbar auto-hide is implemented,
                        // instead of letting the artwork shrink to 0, disable it when
                        // the window height is too small to fit the artwork at the
                        // minimum size. This is because the library might not be easy
                        // to navigate when the window height is too small
                        album_cover.set_height_request(0);
                        album_cover.set_width_request(0);
                        title_label.set_label(&song_info.title);
                        album_label.set_label(&song_info.album);
                        artist_label.set_label(&song_info.artist);

                        song_duration = Duration::from_millis(song_info.duration.mseconds());
                        time_end_label.set_label(&format_duration(&song_duration));

                        if song_info.lyrics.is_empty() {
                            lyrics_label.set_label("Lyrics not available");
                        } else {
                            lyrics_label.set_label(&song_info.lyrics);
                        }
                    }
                    UpdateUI::PlayerTime(time) => {
                        let Some(time_ms) = time.map(gst::ClockTime::mseconds) else {
                            seek_bar.set_sensitive(false);
                            seek_bar.set_child_visible(false);
                            time_cur_label.set_label("-:--");
                            seek_bar.set_value(0.0);
                            continue;
                        };

                        seek_bar.set_sensitive(true);
                        seek_bar.set_child_visible(true);
                        time_cur_label.set_label(&format_duration(&Duration::from_millis(time_ms)));
                        seek_bar.set_value(time_ms as f64 / song_duration.as_millis() as f64);
                    }
                    UpdateUI::Progress(progress) => {
                        if let Some(progress) = progress {
                            progress_bar.set_visible(true);
                            progress_bar.set_fraction(progress);
                        } else {
                            progress_bar.set_visible(false);
                        }
                    }
                }
            }
        },
    ));
}
