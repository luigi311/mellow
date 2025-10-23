use adw::ApplicationWindow;
use adw::subclass::prelude::*;
use adw::{gio, glib};
use gio::Settings;
use glib::subclass::InitializingObject;
use gtk::prelude::{ButtonExt, RangeExt, WidgetExt};
use gtk::{CompositeTemplate, gdk};
use std::rc::Rc;

use std::cell::OnceCell;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::format_duration;
use crate::library::SongInfo;
use crate::player::PlayerRequest;
use crate::player::song_queue::SongQueue;
use crate::ui::UpdateUI;
use gst::{ClockTime, State};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/window.ui")]
pub struct Window {
    #[template_child]
    pub progress_bar: TemplateChild<gtk::ProgressBar>,

    #[template_child]
    pub album_cover: TemplateChild<gtk::Picture>,
    #[template_child]
    pub song_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub album_title: TemplateChild<gtk::Label>,
    #[template_child]
    pub artist_name: TemplateChild<gtk::Label>,

    #[template_child]
    pub media_controls: TemplateChild<gtk::Box>,
    #[template_child]
    pub pause_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub seek_bar: TemplateChild<gtk::Scale>,
    #[template_child]
    pub time_cur_label: TemplateChild<gtk::Label>,
    #[template_child]
    pub time_end_label: TemplateChild<gtk::Label>,

    #[template_child]
    pub view_stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    pub view_switcher_bar: TemplateChild<adw::ViewSwitcherBar>,

    #[template_child]
    info_song_title: TemplateChild<gtk::Label>,
    #[template_child]
    info_lyrics: TemplateChild<gtk::Label>,

    // TODO: Save/load settings
    #[template_child]
    pub settings_volume: TemplateChild<gtk::Scale>,
    #[template_child]
    pub settings_shuffle: TemplateChild<adw::SwitchRow>,
    #[template_child]
    pub settings_repeat: TemplateChild<adw::SwitchRow>,
    #[template_child]
    pub settings_gapless: TemplateChild<adw::SwitchRow>,

    pub settings: OnceCell<Settings>,
    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
}

#[gtk::template_callbacks]
impl Window {
    #[template_callback]
    pub fn handle_skip_prev(&self) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::SkipPrevious)
            .unwrap();
    }
    #[template_callback]
    pub fn handle_play_pause(&self) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::PlayOrPause)
            .unwrap();
    }
    #[template_callback]
    pub fn handle_skip_next(&self) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::SkipNext)
            .unwrap();
    }
    #[template_callback]
    pub fn handle_add_library(&self) {
        println!("TODO: handle_add_library(): Open directory dialog");
    }

    fn connect_closures(&self) {
        let player_tx = self.player_tx.get().unwrap().clone();

        self.seek_bar.connect_change_value({
            let player_tx = player_tx.clone();
            move |_, _, value| {
                player_tx.send(PlayerRequest::Seek(value)).unwrap();
                glib::Propagation::Proceed
            }
        });
        self.settings_volume.connect_change_value({
            let player_tx = player_tx.clone();
            move |_, _, value| {
                player_tx.send(PlayerRequest::SetVolume(value)).unwrap();
                glib::Propagation::Proceed
            }
        });

        self.settings_shuffle.connect_active_notify({
            let player_tx = player_tx.clone();
            move |switch| {
                player_tx
                    .send(PlayerRequest::SetShuffle(switch.is_active()))
                    .unwrap();
            }
        });
        self.settings_repeat.connect_active_notify({
            let player_tx = player_tx.clone();
            move |switch| {
                player_tx
                    .send(PlayerRequest::SetRepeat(switch.is_active()))
                    .unwrap();
            }
        });
        self.settings_gapless.connect_active_notify({
            let player_tx = player_tx.clone();
            move |switch| {
                player_tx
                    .send(PlayerRequest::SetGapless(switch.is_active()))
                    .unwrap();
            }
        });
    }

    pub async fn event_handler(&self, mut ui_rx: tokio_mpsc::Receiver<UpdateUI>) {
        self.connect_closures();
        let mut song_duration = Duration::default();

        loop {
            let Some(response) = ui_rx.recv().await else {
                continue;
            };

            match response {
                UpdateUI::PlayerState(state, interactive) => {
                    self.update_state(state, interactive);
                }
                UpdateUI::SongInfo(song_info) => {
                    self.update_song_info(song_info, &mut song_duration);
                }
                UpdateUI::PlayerTime(time) => {
                    self.update_time(time, song_duration.as_millis() as f64);
                }
                UpdateUI::Progress(progress) => {
                    self.update_progress(progress);
                }
            }
        }
    }

    fn update_state(&self, state: State, interactive: bool) {
        self.pause_button.set_icon_name(match state {
            State::Playing => "media-playback-pause-symbolic",
            _ => "media-playback-start-symbolic",
        });
        self.media_controls.set_sensitive(interactive);
    }

    fn update_song_info(&self, song_info: Option<Arc<SongInfo>>, song_duration: &mut Duration) {
        let Some(song_info) = song_info else { return };

        if let Some(artwork) = song_info.artwork.as_ref() {
            self.album_cover.set_paintable(Some(artwork));
        } else {
            self.album_cover
                .set_paintable(Some(&gdk::Paintable::new_empty(1, 1)));
        }

        self.album_cover.set_width_request(0);
        self.album_cover.set_height_request(0);
        self.song_title.set_label(&song_info.title);
        self.album_title.set_label(&song_info.album);
        self.artist_name.set_label(&song_info.artist);

        let duration_ms = song_info.duration.mseconds();
        *song_duration = Duration::from_millis(duration_ms);
        if duration_ms > 0 {
            self.time_end_label
                .set_label(&format_duration(song_duration));
        } else {
            self.time_end_label.set_label("-:--");
        }

        self.info_song_title.set_label(&song_info.title);
        if song_info.lyrics.is_empty() {
            self.info_lyrics.set_label("Lyrics not available");
        } else {
            self.info_lyrics.set_label(&song_info.lyrics);
        }
    }

    fn update_time(&self, time: Option<ClockTime>, duration: f64) {
        if let Some(time_ms) = time.map(gst::ClockTime::mseconds) {
            self.time_cur_label
                .set_label(&format_duration(&Duration::from_millis(time_ms)));
            self.seek_bar.set_child_visible(true);
            if duration > 0.0 {
                self.seek_bar.set_sensitive(true);
                self.seek_bar.set_value(time_ms as f64 / duration);
            } else {
                self.seek_bar.set_sensitive(false);
                self.seek_bar.set_value(0.0);
            }
        } else {
            self.time_cur_label.set_label("-:--");
            self.seek_bar.set_child_visible(false);
            self.seek_bar.set_sensitive(false);
            self.seek_bar.set_value(0.0);
        }
    }

    fn update_progress(&self, progress: Option<f64>) {
        if let Some(progress) = progress {
            self.progress_bar.set_visible(true);
            self.progress_bar.set_fraction(progress);
        } else {
            self.progress_bar.set_visible(false);
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &str = "MellowWindow";
    type Type = super::Window;
    type ParentType = ApplicationWindow;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}
impl ObjectImpl for Window {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();
        obj.setup_settings();
        obj.load_window_size();

        self.album_cover
            .set_paintable(Some(&gdk::Paintable::new_empty(1, 1)));
    }
}
impl WidgetImpl for Window {}
impl WindowImpl for Window {
    fn close_request(&self) -> glib::Propagation {
        self.obj()
            .save_window_size()
            .expect("Failed to save window state");
        glib::Propagation::Proceed
    }
}
impl ApplicationWindowImpl for Window {}
impl AdwApplicationWindowImpl for Window {}
