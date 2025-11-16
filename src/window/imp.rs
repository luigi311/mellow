use adw::ApplicationWindow;
use adw::{prelude::*, subclass::prelude::*};
use glib::clone;
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gdk, gio, glib};

use std::cell::{Cell, OnceCell, RefCell};
use std::sync::{Arc, mpsc};
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::library::SongInfo;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::queue_row::QueueRow;
use crate::song_page::SongPage;
use crate::ui::UpdateUI;
use crate::{approx_eq, format_duration};
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
    #[template_child]
    playing_navigation_view: TemplateChild<adw::NavigationView>,

    // #[template_child]
    // lyrics_page_title: TemplateChild<adw::WindowTitle>,
    #[template_child]
    info_song_title: TemplateChild<gtk::Label>,
    #[template_child]
    info_lyrics: TemplateChild<gtk::Label>,
    #[template_child]
    song_page: TemplateChild<SongPage>,
    #[template_child]
    song_queue_scrolled_window: TemplateChild<gtk::ScrolledWindow>,
    #[template_child]
    song_queue_list_box: TemplateChild<gtk::ListBox>,
    // #[template_child]
    // song_queue_list_view: TemplateChild<gtk::ListView>,
    #[template_child]
    shuffle_toggle: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    repeat_toggle: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    pub settings_volume: TemplateChild<gtk::Scale>,
    #[template_child]
    pub settings_gapless: TemplateChild<adw::SwitchRow>,

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
            .send(PlayerRequest::TogglePlay(None))
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
    pub fn handle_seek(&self, _: gtk::ScrollType, value: f64) -> glib::Propagation {
        if approx_eq(value, self.seek_bar.value()) {
            return glib::Propagation::Stop;
        }
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::Seek(value))
            .unwrap();
        glib::Propagation::Proceed
    }
    #[template_callback]
    pub fn handle_set_volume(&self, _: gtk::ScrollType, value: f64) -> glib::Propagation {
        if approx_eq(value, self.settings_volume.value()) {
            return glib::Propagation::Stop;
        }
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::SetVolume(value * value))
            .unwrap();
        glib::Propagation::Proceed
    }
    #[template_callback]
    pub fn handle_gapless_switch(&self) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::SetGapless(self.settings_gapless.is_active()))
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
    pub fn handle_set_shuffle(&self, toggle_button: &gtk::ToggleButton) {
        self.player_tx
            .get()
            .unwrap()
            .send(PlayerRequest::SetShuffle(toggle_button.is_active()))
            .unwrap();
    }

    fn connect_closures(&self) {
        // Connect the seek bar `release` callback to resume playback after seeking
        let release_seek_bar = gtk::GestureClick::new();
        release_seek_bar.connect_released({
            let player_tx = self.player_tx.get().unwrap().clone();
            move |_, _, _, _| player_tx.send(PlayerRequest::SeekDone).unwrap()
        });

        // As a workaround for `release` not being signaled by `GtkScale`,
        // set propagation phase to `Capture` and add controller to parent
        // Source: https://stackoverflow.com/a/79108304
        release_seek_bar.set_propagation_phase(gtk::PropagationPhase::Capture);
        self.seek_bar
            .parent()
            .unwrap()
            .add_controller(release_seek_bar);

        self.song_page.init(
            self.player_tx.get().unwrap().clone(),
            self.playing_navigation_view.get(),
            self.sheet.get(),
        );
    }

    #[allow(clippy::future_not_send)]
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
                // TODO: Get rid of `UpdateUI::SongInfo` if possible
                UpdateUI::SongInfo => {
                    self.update_song_info(&mut song_duration);
                }
                UpdateUI::PlayerTime(time) => {
                    self.update_time(time, song_duration.as_millis() as f64);
                }
                UpdateUI::Shuffle(shuffle) => self.update_shuffle(shuffle),
                UpdateUI::Repeat(repeat) => self.update_repeat(repeat),
                UpdateUI::SongQueue(queue) => self.update_song_queue(Some(queue)),
                UpdateUI::QueueIndex(index) => self.update_song_index(index),
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

    fn update_song_info(&self, song_duration: &mut Duration) {
        println!("update_song_info()");
        let queue = self.song_queue.borrow();
        if queue.is_empty() {
            return;
        }
        let index = self.song_queue_index.get();
        let song = &queue[index];
        if song.is_stopper() {
            return;
        }
        let mut song = song.as_song();
        let mut info = song.info();
        let detailed_info = info.take_detailed();
        let song_info = info.basic();

        if let Some(artwork) = detailed_info.artwork.as_ref() {
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

        // self.lyrics_page_title.set_title(&song_info.title);
        // self.lyrics_page_title.set_subtitle(&song_info.artist);
        self.info_song_title.set_label(&song_info.title);
        if detailed_info.lyrics.is_empty() {
            self.info_lyrics.set_label("Lyrics not available");
        } else {
            self.info_lyrics.set_label(&detailed_info.lyrics);
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

    fn update_shuffle(&self, shuffle: bool) {
        self.shuffle_toggle.set_icon_name(match shuffle {
            true => "media-playlist-shuffle-symbolic",
            false => "media-playlist-consecutive-symbolic",
        });
        self.shuffle_toggle.set_active(shuffle);
    }

    fn update_repeat(&self, repeat: bool) {
        self.repeat_toggle.set_active(repeat);
    }
    fn update_song_index(&self, index: usize) {
        println!("update_song_index()");
        self.song_queue_index.set(index);
        self.update_song_queue(None);
    }
    fn update_song_queue(&self, queue: Option<Box<[QueueItem]>>) {
        println!("update_song_queue()");
        if let Some(queue) = queue {
            let _ = self.song_queue.replace(queue);
        }

        // TODO: Display the list properly (model/factory/view)
        // TODO: Support removing queue items
        // TODO: Support reordering queue items
        // TODO: Support inserting stoppers
        // TODO: Support rating/tagging songs (AdwExpanderRow/subpage/context menu)
        // TODO: Display the entire queue
        self.song_queue_list_box.remove_all();
        let index = self.song_queue_index.get();
        let start = index.saturating_sub(10);
        let end = (index + 15).min(self.song_queue.borrow().len());
        let queue = self.song_queue.borrow();
        for i in start..end {
            match &queue[i] {
                QueueItem::Song(song) => {
                    let is_playing = i == index;
                    let queue_entry = QueueRow::default();

                    let mut song = song.lock().unwrap();
                    let mut info = song.info();

                    let song_info = info.basic();
                    let song_title = song_info.title.clone();
                    let album_title = song_info.album.clone();
                    let artist_name = song_info.artist.clone();

                    queue_entry.set_title(&song_title);
                    queue_entry.set_subtitle(&album_title);
                    if is_playing {
                        queue_entry.add_css_class("heading");
                        queue_entry.add_css_class("card");
                    }

                    // TODO: Cached low-res album covers
                    let detailed_info = info.detailed();
                    if let Some(artwork) = detailed_info.artwork.as_ref() {
                        queue_entry.set_prefix_image(artwork);
                    } else {
                        queue_entry.set_prefix_image(&gdk::Paintable::new_empty(1, 1));
                    }

                    queue_entry.connect_activated({
                        clone!(
                            #[weak(rename_to=song_page)]
                            self.song_page,
                            #[weak(rename_to=navigation)]
                            self.playing_navigation_view,
                            move |_| {
                                navigation.push_by_tag("info");
                                song_page.set_info(i, &song_title, &album_title, &artist_name);
                            }
                        )
                    });

                    self.song_queue_list_box.append(&queue_entry);
                }
                QueueItem::Stopper => {
                    let queue_entry = QueueRow::default();

                    queue_entry.set_title("Pause");
                    queue_entry.add_css_class("heading");
                    queue_entry.add_css_class("dimmed");

                    // IDEA: Draw a pause icon in place of the album cover
                    // queue_entry.set_prefix_image();

                    // TODO: Open a page for stoppers as well
                    // TODO: Allow removing stoppers
                    // TODO: Allow reordering stoppers
                    // queue_entry.connect_activated({
                    //     let player_tx = self.player_tx.get().unwrap().clone();
                    //     move |_| player_tx.send(PlayerRequest::SkipTo(i)).unwrap()
                    // });

                    self.song_queue_list_box.append(&queue_entry);
                }
            }
        }
        let new_value = (index - start) * 48;
        self.song_queue_scrolled_window
            .vadjustment()
            .set_value(new_value as f64);

        // Garbage collection
        for (index, item) in queue.iter().enumerate() {
            if !(start..end).contains(&index) {
                if let QueueItem::Song(song) = item {
                    let _ = song.lock().map(|mut song| {
                        song.info().unload_detailed();
                    });
                }
            }
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

        class.install_action_async("win.add_library", None, async |window, _, _| {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type("inode/directory");
            let library_picker = gtk::FileDialog::builder()
                .modal(true)
                .default_filter(&filter)
                .accept_label("Add Library")
                .initial_folder(&gio::File::for_path(
                    glib::user_special_dir(glib::UserDirectory::Music)
                        .unwrap_or_else(glib::current_dir),
                ))
                .build();

            if let Ok(dir) = library_picker.select_folder_future(Some(&window)).await {
                println!("TODO: Add library");
                dbg!(dir.path());
            }
        });
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

        self.album_cover
            .set_paintable(Some(&gdk::Paintable::new_empty(1, 1)));
    }
}
impl WindowImpl for Window {
    fn close_request(&self) -> glib::Propagation {
        self.obj()
            .save_settings()
            .expect("Failed to save window state");
        glib::Propagation::Proceed
    }
}
impl WidgetImpl for Window {}
impl ApplicationWindowImpl for Window {}
impl AdwApplicationWindowImpl for Window {}
