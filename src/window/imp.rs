use adw::ApplicationWindow;
use adw::{gio, glib};
use adw::{prelude::*, subclass::prelude::*};
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gdk};

use std::cell::{Cell, OnceCell, RefCell};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::format_duration;
use crate::library::SongInfo;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::UpdateUI;
use gst::{ClockTime, State};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/window.ui")]
pub struct Window {
    #[template_child]
    progress_bar: TemplateChild<gtk::ProgressBar>,

    #[template_child]
    album_cover: TemplateChild<gtk::Picture>,
    #[template_child]
    song_title: TemplateChild<gtk::Label>,
    #[template_child]
    album_title: TemplateChild<gtk::Label>,
    #[template_child]
    artist_name: TemplateChild<gtk::Label>,

    #[template_child]
    media_controls: TemplateChild<gtk::Box>,
    #[template_child]
    pause_button: TemplateChild<gtk::Button>,
    #[template_child]
    seek_bar: TemplateChild<gtk::Scale>,
    #[template_child]
    time_cur_label: TemplateChild<gtk::Label>,
    #[template_child]
    time_end_label: TemplateChild<gtk::Label>,

    #[template_child]
    sheet: TemplateChild<adw::BottomSheet>,
    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,

    // #[template_child]
    // lyrics_page_title: TemplateChild<adw::WindowTitle>,
    #[template_child]
    info_song_title: TemplateChild<gtk::Label>,
    #[template_child]
    info_lyrics: TemplateChild<gtk::Label>,
    #[template_child]
    playing_song_title: TemplateChild<gtk::Label>,
    #[template_child]
    playing_album_title: TemplateChild<gtk::Label>,
    #[template_child]
    playing_artist_name: TemplateChild<gtk::Label>,
    #[template_child]
    song_queue_group: TemplateChild<adw::PreferencesGroup>,

    // TODO: Save/load settings
    // TODO: Keep switch positions (etc) in sync with the player settings (where needed)
    #[template_child]
    settings_volume: TemplateChild<gtk::Scale>,
    #[template_child]
    settings_gapless: TemplateChild<adw::SwitchRow>,

    pub settings: OnceCell<gio::Settings>,
    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,

    song_queue: RefCell<Box<[QueueItem]>>,
    song_queue_index: Cell<usize>,
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
    pub fn handle_set_repeat(&self, toggle_button: &gtk::ToggleButton) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::SetRepeat(toggle_button.is_active()))
            .unwrap();
    }
    #[template_callback]
    pub fn handle_set_shuffle(&self, tb: &gtk::ToggleButton) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::SetShuffle(tb.is_active()))
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
                UpdateUI::SongQueue(queue) => self.update_song_queue(queue),
                UpdateUI::QueueIndex(index) => self.song_queue_index.set(index),
                UpdateUI::Progress(progress) => self.update_progress(progress),
                UpdateUI::OpenLibrary => self.open_library(),
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

        self.playing_song_title.set_label(&song_info.title);
        self.playing_album_title.set_label(&song_info.album);
        self.playing_artist_name.set_label(&song_info.artist);
        // self.lyrics_page_title.set_title(&song_info.title);
        // self.lyrics_page_title.set_subtitle(&song_info.artist);
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

    fn update_song_queue(&self, queue: Box<[QueueItem]>) {
        // TODO: Clear old items when updating
        // TODO: Indicate the currently playing song after each track change
        // TODO: Display the list properly (model/factory/view)
        // TODO: Support removing queue items
        // TODO: Support reordering queue items
        // TODO: Support jumping between songs in the queue
        // TODO: Support inserting stoppers
        // TODO: Support rating/tagging songs (AdwExpanderRow or context menu)
        let _ = self.song_queue.replace(queue);
        // TODO: Display the entire queue
        for i in self.song_queue_index.get().saturating_sub(5)
            ..(self.song_queue_index.get() + 10).min(self.song_queue.borrow().len())
        {
            match &self.song_queue.borrow()[i] {
                QueueItem::Song(song) => {
                    let is_playing = i == self.song_queue_index.get();
                    let mut song = song.lock().unwrap();
                    let song_info = song.get_info_or_assign();
                    let queue_entry = adw::ActionRow::builder()
                        .title_lines(1)
                        .subtitle_lines(1)
                        .use_markup(false)
                        .activatable(true)
                        .build();
                    queue_entry.set_title(&song_info.title);
                    queue_entry.set_subtitle(&song_info.artist);
                    if is_playing {
                        queue_entry.add_css_class("heading");
                        queue_entry.add_css_class("card");
                    }
                    let cover_widget = gtk::Picture::builder()
                        .valign(gtk::Align::Center)
                        .content_fit(gtk::ContentFit::Fill)
                        .margin_top(if is_playing { 4 } else { 8 })
                        .margin_bottom(if is_playing { 4 } else { 8 })
                        .css_classes(["card"])
                        .build();
                    // TODO: Cached low-res album covers
                    if let Some(artwork) = song_info.artwork.as_ref() {
                        cover_widget.set_paintable(Some(artwork));
                    } else {
                        cover_widget.set_paintable(Some(&gdk::Paintable::new_empty(1, 1)));
                    }
                    queue_entry.add_prefix(&cover_widget);

                    self.song_queue_group.add(&queue_entry);
                }
                QueueItem::Stopper => {
                    // TODO: Display stoppers
                }
            }
        }
    }

    fn open_library(&self) {
        self.view_stack.set_visible_child_name("library");
        self.sheet.set_open(true);
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
