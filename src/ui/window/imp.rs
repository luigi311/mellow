use adw::ApplicationWindow;
use adw::{prelude::*, subclass::prelude::*};
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gdk, gio, glib};

use std::cell::{Cell, OnceCell, RefCell};
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::UpdateUI;
use crate::ui::main_player::MainPlayer;
use crate::ui::queue_page::QueuePage;
use crate::ui::rating::Rating;
use crate::ui::song_page::SongPage;
use crate::{approx_eq, format_duration};
use gst::{ClockTime, State};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/window.ui")]
pub struct Window {
    #[template_child]
    progress_bar: TemplateChild<gtk::ProgressBar>,

    #[template_child]
    main_player: TemplateChild<MainPlayer>,

    #[template_child]
    queue_page: TemplateChild<QueuePage>,

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

    fn init_ui_elements(&self) {
        self.main_player.init(self.player_tx.get().unwrap().clone());
        self.queue_page.init(
            self.player_tx.get().unwrap().clone(),
            self.song_page.clone(),
            self.playing_navigation_view.get(),
        );
        self.song_page.init(
            self.player_tx.get().unwrap().clone(),
            self.playing_navigation_view.get(),
            self.sheet.get(),
        );
    }

    #[allow(clippy::future_not_send)]
    pub async fn event_handler(&self, mut ui_rx: tokio_mpsc::Receiver<UpdateUI>) -> ! {
        self.init_ui_elements();

        let mut song_duration = Duration::default();
        loop {
            let Some(response) = ui_rx.recv().await else {
                continue;
            };

            match response {
                UpdateUI::PlayerState(state, interactive) => {
                    self.main_player
                        .set_state(matches!(state, State::Playing), interactive);
                }
                // TODO: Get rid of `UpdateUI::SongInfo` if possible
                UpdateUI::SongInfo => {
                    self.update_song_info(&mut song_duration);
                }
                UpdateUI::PlayerTime(time) => {
                    self.main_player
                        .set_time(time, song_duration.as_millis() as f64);
                }
                UpdateUI::Shuffle(shuffle) => self.queue_page.update_shuffle(shuffle),
                UpdateUI::Repeat(repeat) => self.queue_page.update_repeat(repeat),
                UpdateUI::SongQueue(queue) => self.update_song_queue(Some(queue)),
                UpdateUI::QueueIndex(index) => self.update_song_index(index),
                UpdateUI::Progress(progress) => self.update_progress(progress),
                UpdateUI::OpenLibrary => self.open_library(),
            }
        }
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

        let duration_ms = song_info.duration.mseconds();
        *song_duration = Duration::from_millis(duration_ms);
        self.main_player.set_info(
            &song_info.title,
            &song_info.album,
            &song_info.artist,
            detailed_info.artwork.as_ref(),
            song_duration,
        );

        // self.lyrics_page_title.set_title(&song_info.title);
        // self.lyrics_page_title.set_subtitle(&song_info.artist);
        self.info_song_title.set_label(&song_info.title);
        if detailed_info.lyrics.is_empty() {
            self.info_lyrics.set_label("Lyrics not available");
        } else {
            self.info_lyrics.set_label(&detailed_info.lyrics);
        }
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
        self.queue_page
            .update_song_queue(self.song_queue.borrow(), self.song_queue_index.get());
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
        MainPlayer::static_type();
        Rating::static_type();

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
impl WidgetImpl for Window {
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.parent_size_allocate(width, height, baseline);
        self.main_player.update_spacing(height - 48);
    }
}
impl ApplicationWindowImpl for Window {}
impl AdwApplicationWindowImpl for Window {}
