use adw::ApplicationWindow;
use adw::{prelude::*, subclass::prelude::*};
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gio, glib};
use std::cell::{Cell, OnceCell, RefCell};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::MUSIC_DIR;
use crate::excuses::{ACTION_ERR, EXP_INIT, EXP_RX};
use crate::library::album::AlbumMutex;
use crate::library::artist::ArtistMutex;
use crate::library::song::SongMutex;
use crate::library::{Albums, Artists, LIBRARY_TX, Library, LibraryRequest, Songs, ToQueue};
use crate::player::queue_item::QueueItem;
use crate::ui::album_page::AlbumPage;
use crate::ui::albums_page::AlbumsPage;
use crate::ui::artist_page::ArtistPage;
use crate::ui::artists_page::ArtistsPage;
use crate::ui::library_page::LibraryPage;
use crate::ui::lyrics_page::LyricsPage;
use crate::ui::main_player::MainPlayer;
use crate::ui::queue_page::QueuePage;
use crate::ui::queue_subpage::QueueSubpage;
use crate::ui::rating::Rating;
use crate::ui::settings_page::SettingsPage;
use crate::ui::song_page::SongPage;
use crate::ui::songs_page::SongsPage;
use crate::ui::{UI_TX, UpdateUI};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/window.ui")]
pub struct Window {
    #[template_child]
    pub main_player: TemplateChild<MainPlayer>,

    #[template_child]
    progress_bar: TemplateChild<gtk::ProgressBar>,
    #[template_child]
    bottom_bar: TemplateChild<gtk::Box>,
    #[template_child]
    sheet: TemplateChild<adw::BottomSheet>,
    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,

    // View stack "Library" tab
    #[template_child]
    pub library: TemplateChild<adw::NavigationView>,
    #[template_child]
    pub library_page: TemplateChild<LibraryPage>,
    #[template_child]
    pub songs_page: TemplateChild<SongsPage>,
    #[template_child]
    pub song_page: TemplateChild<SongPage>,
    #[template_child]
    pub albums_page: TemplateChild<AlbumsPage>,
    #[template_child]
    pub album_page: TemplateChild<AlbumPage>,
    #[template_child]
    pub artists_page: TemplateChild<ArtistsPage>,
    #[template_child]
    pub artist_page: TemplateChild<ArtistPage>,

    // View stack "Playing" tab
    #[template_child]
    pub playing: TemplateChild<adw::NavigationView>,
    #[template_child]
    queue_page: TemplateChild<QueuePage>,
    #[template_child]
    queue_song_page: TemplateChild<QueueSubpage>,
    #[template_child]
    lyrics_page: TemplateChild<LyricsPage>,

    // View stack "Settings" tab
    #[template_child]
    pub settings_page: TemplateChild<SettingsPage>,

    pub settings: OnceCell<gio::Settings>,

    pub song_queue: RefCell<Box<[QueueItem]>>,
    pub song_queue_index: Cell<usize>,

    pub songs: RefCell<Songs>,
    pub albums: RefCell<Albums>,
    pub artists: RefCell<Artists>,
}

impl Window {
    pub fn init_ui_elements(&self) {
        self.main_player.init();
        self.queue_page.init(self.queue_song_page.get());
        self.songs_page.init_search();
        self.albums_page.init_search();
        self.artists_page.init_search();
        self.settings_page
            .init(self.bottom_bar.get(), self.sheet.get());
    }

    #[allow(clippy::future_not_send)]
    pub async fn event_handler(&self, mut ui_rx: tokio_mpsc::UnboundedReceiver<UpdateUI>) -> ! {
        let mut song_duration = Duration::default();
        loop {
            match ui_rx.recv().await.unwrap() {
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
                UpdateUI::RedrawQueue => self.update_song_index(self.song_queue_index.get()),
                UpdateUI::QueueSupbage(index) => self.open_queue_subpage(index),
                UpdateUI::Shuffle(shuffle) => self.queue_page.update_shuffle(shuffle),
                UpdateUI::Repeat(repeat) => self.queue_page.update_repeat(repeat),
                UpdateUI::Progress(progress) => self.update_progress(progress),

                UpdateUI::LibraryDirs(dirs) => self.set_library_dirs(&dirs),
                UpdateUI::LibrarySongs(songs) => self.load_library_songs(&songs),
                UpdateUI::LibraryAlbums(albums) => self.load_library_albums(&albums),
                UpdateUI::LibraryArtists(artists) => self.load_library_artists(&artists),

                UpdateUI::ArtistPageByIndex(index) => self.open_artist_page_by_index(index),
                UpdateUI::ArtistPage(artist) => self.open_artist_page(&artist),
                UpdateUI::AlbumPageByIndex(index) => self.open_album_page_by_index(index),
                UpdateUI::AlbumPage(album) => self.open_album_page(&album),
                UpdateUI::SongPageByIndex(index) => self.open_song_page_by_index(index),
                UpdateUI::SongPage(context) => {
                    self.open_song_page(context.0, context.1, context.2);
                }

                UpdateUI::FocusLibrary => self.focus_library(),
                UpdateUI::FocusPlaying => self.focus_playing(),
                UpdateUI::FocusSettings => self.focus_settings(),
                UpdateUI::OpenSheet(open) => self.open_sheet(open),
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
        let stop_after = index + 1 < queue.len() && queue[index + 1].is_stopper();
        self.queue_song_page.set_stop_after(stop_after);

        let song = &queue[index];
        if song.is_stopper() {
            return;
        }

        let song = match song {
            QueueItem::Song(song) => song,
            QueueItem::Stopper => unreachable!(),
        };
        let mut song_locked = song.lock().unwrap();
        let mut info = song_locked.info();

        let song_info = info.basic();
        let (title, album, artist, duration_ms) = (
            song_info.title.clone(),
            song_info.album.clone(),
            song_info.artist.clone(),
            song_info.duration.mseconds(),
        );

        let detailed_info = info.inspect_detailed();
        let artwork = match detailed_info {
            Some(detailed) => {
                self.lyrics_page.set_content(&title, &detailed.lyrics);
                detailed.artwork.as_ref()
            }
            None => {
                drop(song_locked);
                let song = Arc::clone(song);
                let ui_tx = UI_TX.get().expect(EXP_INIT);
                let load_artwork_handle = thread::spawn(move || {
                    song.lock().unwrap().info().load_detailed();
                    ui_tx.send(UpdateUI::SongInfo).expect(EXP_RX);
                });
                Library::run_task(LIBRARY_TX.get().expect(EXP_RX), move || {
                    load_artwork_handle.join().unwrap();
                });
                None
            }
        };

        *song_duration = Duration::from_millis(duration_ms);
        self.main_player
            .set_info(&title, &album, &artist, artwork, song_duration);

        match artwork {
            Some(artwork) => self.settings_page.set_background_from_artwork(artwork),
            None => self.settings_page.disable_background_color(),
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
    fn open_queue_subpage(&self, index: usize) {
        let queue = self.song_queue.borrow();
        let stop_after = index + 1 < queue.len() && queue[index + 1].is_stopper();
        self.queue_song_page
            .activate_action("ui.playing_nav_push", Some(&"info".to_variant()))
            .expect(ACTION_ERR);
        self.queue_song_page.update(
            index,
            Arc::clone(match &queue[index] {
                QueueItem::Song(song) => song,
                _ => unreachable!(),
            }),
        );
        self.queue_song_page.set_stop_after(stop_after);
    }

    fn update_progress(&self, progress: Option<f64>) {
        self.library_page.update_progress(progress);
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

    fn load_library_songs(&self, songs: &Songs) {
        self.library_page.set_empty(songs.is_empty());
        self.songs.replace(songs.clone());
        self.songs_page.load_songs(songs);
    }
    fn load_library_albums(&self, albums: &Albums) {
        self.albums.replace(albums.clone());
        self.albums_page.load_albums(albums);
    }
    fn load_library_artists(&self, artists: &Artists) {
        self.artists.replace(artists.clone());
        self.artists_page.load_artists(artists);
    }

    // TODO: Reset the scroll position when opening song/album/artist page

    fn open_song_page_by_index(&self, index: usize) {
        let songs: Songs = self.songs.borrow().clone();
        self.open_song_page(index, Arc::clone(&songs[index]), Box::new(songs));
    }
    fn open_song_page(&self, index: usize, song: SongMutex, to_queue: Box<dyn ToQueue + Send>) {
        self.song_page.update(index, song, to_queue);
        self.library.push_by_tag("song");
    }
    fn open_artist_page_by_index(&self, index: usize) {
        self.open_artist_page(&self.artists.borrow()[index]);
    }
    fn open_artist_page(&self, artist: &ArtistMutex) {
        self.artist_page.update(artist);
        self.library.push_by_tag("artist");
    }
    fn open_album_page_by_index(&self, index: usize) {
        self.open_album_page(&self.albums.borrow()[index]);
    }
    fn open_album_page(&self, album: &AlbumMutex) {
        self.album_page.update(album);
        self.library.push_by_tag("album");
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &str = "MellowWindow";
    type Type = super::Window;
    type ParentType = ApplicationWindow;

    fn class_init(class: &mut Self::Class) {
        LibraryPage::static_type();
        AlbumPage::static_type();
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
                let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
                library_tx
                    .send(LibraryRequest::AddLibrary(
                        dir.path().unwrap().to_str().unwrap().into(),
                    ))
                    .expect(EXP_RX);
            }
        });

        class.install_action_async("win.queue_from_disk", None, async |window, _, _| {
            let filter = gtk::FileFilter::new();
            filter.add_mime_type("audio/*");
            filter.add_mime_type("inode/directory");
            let file_picker = gtk::FileDialog::builder()
                .modal(true)
                .default_filter(&filter)
                .accept_label("Play Now")
                .initial_folder(&gio::File::for_path(MUSIC_DIR.get().expect(EXP_INIT)))
                .build();

            // TODO: If possible, allow files OR folders
            if let Ok(dirs) = file_picker.open_multiple_future(Some(&window)).await {
                let mut paths = vec![];
                let mut index = 0;
                while let Some(path) = dirs.item(index) {
                    paths.push(
                        path.downcast::<gio::File>()
                            .unwrap()
                            .path()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string(),
                    );
                    index += 1;
                }
                dbg!(&paths);
                let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
                library_tx
                    .send(LibraryRequest::QueueFromPaths(paths.into()))
                    .expect(EXP_RX);
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
        obj.setup_actions();
        obj.setup_drag_and_drop();
    }
}
impl WindowImpl for Window {
    fn close_request(&self) -> glib::Propagation {
        self.obj()
            .save_state()
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
