use adw::{prelude::*, subclass::prelude::*};
use glib::Object;
use gtk::Orientation;
use gtk::{gdk, glib};

use gst::ClockTime;
use std::{sync::mpsc, time::Duration};

use crate::format_duration;
use crate::player::PlayerRequest;

mod imp;

glib::wrapper! {
    pub struct MainPlayer(ObjectSubclass<imp::MainPlayer>)
        @extends gtk::Box, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for MainPlayer {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl MainPlayer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&self, player_tx: mpsc::SyncSender<PlayerRequest>) {
        let ui = self.imp();
        ui.player_tx.set(player_tx.clone()).unwrap();

        // Connect the seek bar `release` callback to resume playback after seeking
        // As a workaround for `release` not being signaled by `GtkScale`,
        // set propagation phase to `Capture` and add controller to parent
        // Source: https://stackoverflow.com/a/79108304
        let release_seek_bar = gtk::GestureClick::builder()
            .propagation_phase(gtk::PropagationPhase::Capture)
            .build();
        release_seek_bar.connect_released({
            // let player_tx = player_tx.clone();
            move |_, _, _, _| player_tx.send(PlayerRequest::SeekDone).unwrap()
        });
        ui.seek_bar
            .parent()
            .unwrap()
            .add_controller(release_seek_bar);
    }

    pub fn set_info(
        &self,
        song: &str,
        album: &str,
        artist: &str,
        artwork: Option<&gdk::Texture>,
        song_duration: &Duration,
    ) {
        let ui = self.imp();

        if artwork.is_some() {
            ui.album_cover.set_paintable(artwork);
        } else {
            ui.album_cover
                .set_paintable(Some(&gdk::Paintable::new_empty(1, 1)));
        }
        ui.album_cover.set_width_request(0);
        ui.album_cover.set_height_request(0);

        ui.song_title.set_label(song);
        ui.album_title.set_label(album);
        ui.artist_name.set_label(artist);

        if !song_duration.is_zero() {
            ui.time_end_label.set_label(&format_duration(song_duration));
        } else {
            ui.time_end_label.set_label("-:--");
        }
    }

    pub fn set_state(&self, playing: bool, interactive: bool) {
        let ui = self.imp();
        ui.pause_button.set_icon_name(match playing {
            true => "media-playback-pause-symbolic",
            false => "media-playback-start-symbolic",
        });
        ui.media_controls.set_sensitive(interactive);
    }

    pub fn set_time(&self, time: Option<ClockTime>, duration_ms: f64) {
        let ui = self.imp();
        if let Some(time_ms) = time.map(gst::ClockTime::mseconds) {
            ui.time_cur_label
                .set_label(&format_duration(&Duration::from_millis(time_ms)));
            ui.seek_bar.set_child_visible(true);
            if duration_ms > 0.0 {
                ui.seek_bar.set_sensitive(true);
                ui.seek_bar.set_value(time_ms as f64 / duration_ms);
            } else {
                ui.seek_bar.set_sensitive(false);
                ui.seek_bar.set_value(0.0);
            }
        } else {
            ui.time_cur_label.set_label("-:--");
            ui.seek_bar.set_child_visible(false);
            ui.seek_bar.set_sensitive(false);
            ui.seek_bar.set_value(0.0);
        }
    }

    /// Sets main player spacing based on available space
    pub fn update_spacing(&self, height: i32) {
        const SPACERS: i32 = 2;
        const WITH_OUTER: i32 = SPACERS + 3;
        let headroom = height + self.spacing() * SPACERS
            - self.size(Orientation::Vertical)
            - self.margin_top()
            - self.margin_bottom();
        self.set_spacing((headroom / WITH_OUTER).max(6));
    }
}
