use adw::ApplicationWindow;
use adw::{prelude::*, subclass::prelude::*};
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gdk, gio, glib};
use std::cell::{Cell, OnceCell, RefCell};
use std::fs;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::LibraryRequest;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::UpdateUI;
use crate::ui::library_albums_page::LibraryAlbumsPage;
use crate::ui::library_artists_page::LibraryArtistsPage;
use crate::ui::library_home_page::LibraryHomePage;
use crate::ui::library_songs_page::LibrarySongsPage;
use crate::ui::lyrics_page::LyricsPage;
use crate::ui::main_player::MainPlayer;
use crate::ui::queue_page::QueuePage;
use crate::ui::queue_song_page::QueueSongPage;
use crate::ui::rating::Rating;
use crate::ui::settings_page::SettingsPage;
use crate::{CONFIG_DIR, MUSIC_DIR};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/window.ui")]
pub struct Window {
    #[template_child]
    pub main_player: TemplateChild<MainPlayer>,

    #[template_child]
    progress_bar: TemplateChild<gtk::ProgressBar>,
    #[template_child]
    bottom_bar: TemplateChild<gtk::CenterBox>,
    #[template_child]
    sheet: TemplateChild<adw::BottomSheet>,
    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,

    // View stack "Library" tab
    #[template_child]
    pub library_songs_page: TemplateChild<LibrarySongsPage>,
    #[template_child]
    pub library_albums_page: TemplateChild<LibraryAlbumsPage>,
    #[template_child]
    pub library_artists_page: TemplateChild<LibraryArtistsPage>,

    // View stack "Playing" tab
    #[template_child]
    queue_page: TemplateChild<QueuePage>,
    #[template_child]
    queue_song_page: TemplateChild<QueueSongPage>,
    #[template_child]
    lyrics_page: TemplateChild<LyricsPage>,
    #[template_child]
    pub playing_navigation_view: TemplateChild<adw::NavigationView>,

    // View stack "Settings" tab
    #[template_child]
    pub settings_page: TemplateChild<SettingsPage>,

    pub settings: OnceCell<gio::Settings>,
    pub library_tx: OnceCell<mpsc::SyncSender<LibraryRequest>>,
    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
    pub css_provider: OnceCell<gtk::CssProvider>,

    song_queue: RefCell<Box<[QueueItem]>>,
    song_queue_index: Cell<usize>,
}

impl Window {
    pub fn init_ui_elements(&self) {
        let player_tx = self.player_tx.get().expect(EXP_INIT).clone();
        let library_tx = self.library_tx.get().expect(EXP_INIT).clone();

        // Main Player
        self.main_player.init(player_tx.clone());

        // Library
        self.library_songs_page
            .init(library_tx.clone(), player_tx.clone());
        self.library_albums_page
            .init(library_tx.clone(), player_tx.clone());
        self.library_artists_page
            .init(library_tx.clone(), player_tx.clone());

        // Queue Page & Subpages
        self.queue_page
            .init(player_tx.clone(), self.queue_song_page.get());
        self.queue_song_page.init(player_tx.clone());

        // Settings Page
        self.settings_page.init(player_tx, library_tx);
    }

    #[allow(clippy::future_not_send)]
    pub async fn event_handler(&self, mut ui_rx: tokio_mpsc::Receiver<UpdateUI>) -> ! {
        let mut song_duration = Duration::default();
        loop {
            let Some(response) = ui_rx.recv().await else {
                continue;
            };

            match response {
                UpdateUI::PlayerState(playing, interactive) => {
                    self.main_player.set_state(playing, interactive);
                }
                UpdateUI::PlayerTime(time) => {
                    self.main_player
                        .set_time(time, song_duration.as_millis() as f64);
                }
                UpdateUI::SongInfo => self.update_song_info(&mut song_duration),
                UpdateUI::NewQueue(queue) => self.update_song_queue(Some(queue)),
                UpdateUI::QueueIndex(index) => self.update_song_index(index),
                UpdateUI::Shuffle(shuffle) => self.queue_page.update_shuffle(shuffle),
                UpdateUI::Repeat(repeat) => self.queue_page.update_repeat(repeat),
                UpdateUI::Progress(progress) => self.update_progress(progress),
                UpdateUI::LibraryDirs(dirs) => self.set_library_dirs(&dirs),
                UpdateUI::LibrarySongs(songs) => self.library_songs_page.load_songs(&songs),
                UpdateUI::LibraryAlbums(albums) => self.library_albums_page.load_albums(&albums),
                UpdateUI::LibraryArtists(artists) => {
                    self.library_artists_page.load_artists(&artists);
                }
                UpdateUI::FocusLibrary => self.focus_library(),
                UpdateUI::FocusPlaying => self.focus_playing(),
                UpdateUI::FocusSettings => self.focus_settings(),
                UpdateUI::OpenSheet(open) => self.open_sheet(open),
            }
        }
    }

    fn set_background_color(&self, r: u8, g: u8, b: u8) {
        let css_provider = self.css_provider.get().expect(EXP_INIT);
        if let Some(display) = gdk::Display::default() {
            gtk::style_context_add_provider_for_display(&display, css_provider, 210);
        }
        css_provider.load_from_string(&format!(
            ".window {{
                 background-color: rgba({r}, {g}, {b}, 1);
                 border-top: 0px none;
                 border-left: 0px none;
                 border-right: 0px none;
                 border-bottom: 0px none;
             }}"
        ));
        if !self.sheet.has_css_class("window") {
            self.sheet.add_css_class("window");
            self.bottom_bar.add_css_class("window");
        }
    }

    fn reset_background_color(&self) {
        self.sheet.remove_css_class("window");
        self.bottom_bar.remove_css_class("window");
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
        self.lyrics_page
            .set_content(&song_info.title, &detailed_info.lyrics);

        if let Some(artwork) = detailed_info.artwork {
            // TODO: Set window background to match artwork colors
            self.set_background_color(16, 16, 16);
        } else {
            self.reset_background_color();
        }
    }

    fn update_song_index(&self, index: usize) {
        println!("update_song_index({index})");
        self.song_queue_index.set(index);
        // FIX: Toggling shuffle mode calls `update_song_queue()` twice
        // NOTE: This call is only needed so the highlighted queue item
        // is updated, and would not be necessary with a proper list view
        self.update_song_queue(None);
    }
    fn update_song_queue(&self, queue: Option<Box<[QueueItem]>>) {
        match queue {
            Some(queue) => {
                println!("update_song_queue(Some(…)): {} items", queue.len());
                self.song_queue.replace(queue);
            }
            None => println!("update_song_queue(None)"),
        }
        self.queue_page
            .update_song_queue(&self.song_queue.borrow(), self.song_queue_index.get());
    }

    fn update_progress(&self, progress: Option<f64>) {
        if let Some(progress) = progress {
            self.progress_bar.set_visible(true);
            self.progress_bar.set_fraction(progress);
        } else {
            self.progress_bar.set_visible(false);
        }
    }

    fn set_library_dirs(&self, dirs: &[String]) {
        self.settings_page.set_directories(dirs);
    }

    fn focus_library(&self) {
        self.view_stack.set_visible_child_name("library");
    }
    fn focus_playing(&self) {
        self.view_stack.set_visible_child_name("playing");
    }
    fn focus_settings(&self) {
        self.view_stack.set_visible_child_name("settings");
    }
    pub fn open_sheet(&self, open: bool) {
        self.sheet.set_open(open);
    }

    fn save_queue(&self) {
        let queue_file = CONFIG_DIR.get().expect(EXP_INIT).to_owned() + "queue";
        // TODO: Also save the shuffled queue and shuffle setting (new file)
        // the file contents could look something like this:
        // /------------------\
        // | True             | <- Whether shuffle mode is on
        // | 50,32,67,4,89,22,| <- Shuffled indexes for the player queue
        // \------------------/
        let shuffled_file = CONFIG_DIR.get().expect(EXP_INIT).to_owned() + "queue_shuffled";
        if self.settings_page.remembers_queue() {
            let _ = fs::write(
                &queue_file,
                self.song_queue_index.get().to_string()
                    + "\n"
                    + self
                        .song_queue
                        .borrow()
                        .iter()
                        .map(|item| match item {
                            QueueItem::Song(song) => song.lock().unwrap().info().file_path() + "\n",
                            QueueItem::Stopper => "".to_string(), // Ignore stoppers
                        })
                        .collect::<String>()
                        .trim(),
            );
        } else {
            let _ = fs::remove_file(&queue_file);
            let _ = fs::remove_file(&shuffled_file);
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &str = "MellowWindow";
    type Type = super::Window;
    type ParentType = ApplicationWindow;

    fn class_init(class: &mut Self::Class) {
        LibraryHomePage::static_type();
        Rating::static_type();

        class.bind_template();

        class.install_action_async("win.add_library", None, async |window, _, _| {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type("inode/directory");
            let library_picker = gtk::FileDialog::builder()
                .modal(true)
                .default_filter(&filter)
                .accept_label("Add Library")
                .initial_folder(&gio::File::for_path(MUSIC_DIR.get().expect(EXP_INIT)))
                .build();

            if let Ok(dir) = library_picker.select_folder_future(Some(&window)).await {
                let library_tx = window.imp().library_tx.get().expect(EXP_INIT);
                library_tx
                    .send(LibraryRequest::AddLibrary(
                        dir.path().unwrap().to_str().unwrap().into(),
                    ))
                    .expect(EXP_RX);
                // TODO: Update incrementally instead of rebuilding
                library_tx.send(LibraryRequest::Rebuild).expect(EXP_RX);
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
        obj.setup_actions();
    }
}
impl WindowImpl for Window {
    fn close_request(&self) -> glib::Propagation {
        self.obj()
            .save_settings()
            .expect("Failed to save window state");

        self.save_queue();

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
