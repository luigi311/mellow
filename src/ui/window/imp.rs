use adw::ApplicationWindow;
use adw::{prelude::*, subclass::prelude::*};
use core::cell::{Cell, OnceCell, RefCell};
use glib::subclass::InitializingObject;
use gtk::{CompositeTemplate, gio, glib};
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use tokio::sync::mpsc as tokio_mpsc;

use crate::MUSIC_DIR;
use crate::excuses::{ACTION_ERR, EXP_INIT, EXP_RX};
use crate::library::{Albums, Artists, SharedAlbum, SharedArtist, SharedSong, Songs, ToQueue};
use crate::library::{LIBRARY_TX, Library, LibraryRequest};
use crate::player::QueueItem;
use crate::ui::{AlbumPage, AlbumsPage, ArtistPage, ArtistsPage, SongPage, SongsPage};
use crate::ui::{LibraryPage, LyricsPage, MainPlayer, SettingsPage, SubpageType};
use crate::ui::{QueuePage, QueueSubpage};
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
    sheet_content: TemplateChild<adw::ToolbarView>,
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
    pub albums_page: TemplateChild<AlbumsPage>,
    #[template_child]
    pub artists_page: TemplateChild<ArtistsPage>,

    pub library_subpages: Rc<RefCell<Vec<SubpageType>>>,
    pub song_pages: Rc<RefCell<Vec<SongPage>>>,
    pub album_pages: Rc<RefCell<Vec<AlbumPage>>>,
    pub artist_pages: Rc<RefCell<Vec<ArtistPage>>>,

    // View stack "Playing" tab
    #[template_child]
    pub playing: TemplateChild<adw::NavigationView>,
    #[template_child]
    pub queue_page: TemplateChild<QueuePage>,
    #[template_child]
    queue_subpage: TemplateChild<QueueSubpage>,
    queue_subpage_visible: Cell<bool>,
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
    #[inline]
    pub fn init_ui_elements(&self, style_manager: adw::StyleManager) {
        self.main_player.init();
        self.queue_page.init(self.queue_subpage.get());
        self.settings_page.init(
            style_manager,
            vec![
                self.sheet.get().upcast::<gtk::Widget>(),
                self.bottom_bar.get().upcast::<gtk::Widget>(),
            ],
            vec![
                self.sheet_content.get().upcast::<gtk::Widget>(),
                (self.main_player.imp().media_controls.get()).upcast::<gtk::Widget>(),
            ],
        );
        self.library.connect_popped(glib::clone!(
            #[weak(rename_to=window)]
            self,
            move |_, _| match window.library_subpages.borrow_mut().pop() {
                Some(SubpageType::Artist) => drop(window.artist_pages.borrow_mut().pop()),
                Some(SubpageType::Album) => drop(window.album_pages.borrow_mut().pop()),
                Some(SubpageType::Song) => drop(window.song_pages.borrow_mut().pop()),
                None => (),
            }
        ));
        self.playing.connect_popped(glib::clone!(
            #[weak(rename_to=window)]
            self,
            move |_, page| if page.downcast_ref::<QueueSubpage>().is_some() {
                window.queue_subpage_visible.set(false);
            },
        ));
    }

    // TODO: Core functionality should be moved into `Application`

    #[allow(clippy::future_not_send)]
    pub async fn event_handler(&self, mut ui_rx: tokio_mpsc::UnboundedReceiver<UpdateUI>) -> ! {
        let mut song_duration_ms = 0;
        loop {
            match ui_rx.recv().await.unwrap() {
                UpdateUI::SongInfo => self.update_song_info(&mut song_duration_ms),
                UpdateUI::PlayerTime(time_ms) => {
                    self.main_player.set_time(time_ms, song_duration_ms as f64);
                }
                UpdateUI::PlayerState(playing, interactive) => {
                    self.main_player.set_state(playing, interactive);
                }

                UpdateUI::SetQueue(queue) => self.update_song_queue(Some(queue)),
                UpdateUI::SetQueueIndex(index) => self.update_song_index(index),
                UpdateUI::OpenQueueSubpage(index) => self.open_queue_subpage(index),
                UpdateUI::CloseQueueSubpage => self.close_queue_subpage(),
                UpdateUI::Shuffle(shuffle) => self.queue_page.update_shuffle(shuffle),
                UpdateUI::Repeat(repeat) => self.queue_page.update_repeat(repeat),
                UpdateUI::Progress(progress) => self.update_progress(progress),

                UpdateUI::SetLibraryDirs(dirs) => self.set_library_dirs(&dirs),
                UpdateUI::SetLibrarySongs(songs) => self.load_library_songs(&songs),
                UpdateUI::SetLibraryAlbums(albums) => self.load_library_albums(&albums),
                UpdateUI::SetLibraryArtists(artists) => self.load_library_artists(&artists),

                UpdateUI::LibrarySongLoaded(index) => self.song_loaded(index),
                UpdateUI::LibraryAlbumLoaded(index) => self.album_loaded(index),
                UpdateUI::LibraryArtistLoaded(index) => self.artist_loaded(index),
                UpdateUI::QueueSongLoaded(index) => self.queue_song_loaded(index),

                UpdateUI::SongPageByIndex(index) => self.open_song_page_by_index(index),
                UpdateUI::SongPage(ctx) => self.open_song_page(ctx.0, ctx.1, ctx.2),
                UpdateUI::AlbumPage(album) => self.open_album_page(&album),
                UpdateUI::ArtistPage(artist) => self.open_artist_page(&artist),

                UpdateUI::FocusLibrary => self.focus_library(),
                UpdateUI::FocusPlaying => self.focus_playing(),
                UpdateUI::FocusSettings => self.focus_settings(),
                UpdateUI::OpenSheet(open) => self.open_sheet(open),

                UpdateUI::RunAction(action) => {
                    WidgetExt::activate_action(&self.main_player.get(), action, None)
                        .expect(ACTION_ERR);
                }

                UpdateUI::Shutdown => loop {
                    // Ignore any further requests without closing the channel
                    ui_rx.recv().await.unwrap();
                    #[cfg(debug_assertions)]
                    println!("Note: UI requests are ignored during shutdown");
                },
            }
        }
    }

    fn update_song_info(&self, song_duration_ms: &mut u64) {
        println!("update_song_info()");
        let queue = self.song_queue.borrow();
        if queue.is_empty() {
            self.settings_page.reset_background_color();
            self.main_player.reset_info();
            return;
        }

        let index = self.song_queue_index.get();
        let stop_after = index + 1 < queue.len() && queue[index + 1].is_stopper();
        self.queue_subpage.set_stop_after(stop_after);

        let song = &queue[index];
        if song.is_stopper() {
            return;
        }

        let song = match song {
            QueueItem::Song(song) => song,
            QueueItem::Stopper => unreachable!(),
        };
        let mut info = song.info();

        let song_info_temp = info.load_basic();
        // SAFETY: `load_basic` ensures the value is `Some`
        let song_info = unsafe { song_info_temp.as_ref().unwrap_unchecked() };
        let (title, album, artist) = (
            song_info.title.clone(),
            song_info.album.clone(),
            song_info.artist.clone(),
        );
        *song_duration_ms = song_info.duration_ms;
        drop(song_info_temp);

        let detailed_info = info.try_inspect_detailed();
        let artwork = match detailed_info
            .as_ref()
            .map_or_else(|_| None, |info| info.as_ref())
        {
            Some(detailed) => {
                self.lyrics_page.set_content(&title, &detailed.lyrics);
                detailed.artwork.as_ref()
            }
            _ => {
                drop(detailed_info);
                let song = Arc::clone(song);
                let ui_tx = UI_TX.get().expect(EXP_INIT);
                let load_artwork_handle = thread::spawn(move || {
                    drop(song.info().load_detailed());
                    ui_tx.send(UpdateUI::SongInfo).expect(EXP_RX);
                });
                Library::run_task(LIBRARY_TX.get().expect(EXP_RX), move || {
                    load_artwork_handle.join().unwrap();
                });
                None
            }
        };

        self.main_player
            .set_info(&title, &album, &artist, artwork, *song_duration_ms);

        match artwork {
            Some(artwork) => self.settings_page.set_background_from_artwork(artwork),
            None => self.settings_page.reset_background_color(),
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
        self.playing.push_by_tag("info");
        self.queue_subpage_visible.set(true);
        let queue = self.song_queue.borrow();
        let stop_after = index + 1 < queue.len() && queue[index + 1].is_stopper();
        self.queue_subpage.update(
            index,
            Arc::clone(match &queue[index] {
                QueueItem::Song(song) => song,
                _ => unreachable!(),
            }),
        );
        self.queue_subpage.set_stop_after(stop_after);
    }
    fn close_queue_subpage(&self) {
        println!("close_queue_subpage()");
        while self.queue_subpage_visible.get() {
            self.playing.pop();
        }
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

    fn song_loaded(&self, index: usize) {
        let song = &self.songs.borrow()[index];
        let info = song.info();
        let Some(ref info) = *info.inspect_detailed() else {
            return;
        };
        self.songs_page.assign_artwork(index, info.artwork.as_ref());
    }
    fn album_loaded(&self, index: usize) {
        let album = &self.albums.borrow()[index];
        let album = album.lock().unwrap();
        let info = album.songs[0].info();
        let Some(ref info) = *info.inspect_detailed() else {
            return;
        };
        self.albums_page
            .assign_artwork(index, info.artwork.as_ref());
    }
    fn artist_loaded(&self, index: usize) {
        self.artists_page.assign_artwork(
            index, None, // TODO: Decide what to show
        );
    }
    fn queue_song_loaded(&self, index: usize) {
        let song_queue = self.song_queue.borrow();
        if index >= song_queue.len() {
            return;
        }
        let QueueItem::Song(song) = &song_queue[index] else {
            return;
        };
        let info = song.info();
        let Some(ref info) = *info.inspect_detailed() else {
            return;
        };
        if info.artwork.is_none() {
            return;
        }
        self.queue_page.assign_artwork(index, info.artwork.as_ref());
    }

    fn open_song_page_by_index(&self, index: usize) {
        let songs: Songs = self.songs.borrow().clone();
        self.open_song_page(index, Arc::clone(&songs[index]), Box::new(songs));
    }
    fn open_song_page(&self, index: usize, song: SharedSong, to_queue: Box<dyn ToQueue + Send>) {
        let song_page = SongPage::new(index, song, to_queue);
        self.library.push(&song_page);
        self.song_pages.borrow_mut().push(song_page);
        self.library_subpages.borrow_mut().push(SubpageType::Song);
    }
    fn open_album_page(&self, album: &SharedAlbum) {
        let _ = self.library.activate_action(
            "menu.album_page_play_mode",
            Some(&"Sequential".to_variant()),
        );
        let album_page = AlbumPage::new(album);
        self.library.push(&album_page);
        self.album_pages.borrow_mut().push(album_page);
        self.library_subpages.borrow_mut().push(SubpageType::Album);
    }
    fn open_artist_page(&self, artist: &SharedArtist) {
        let _ = self.library.activate_action(
            "menu.artist_page_play_mode",
            Some(&"Sequential".to_variant()),
        );
        let artist_page = ArtistPage::new(artist);
        self.library.push(&artist_page);
        self.artist_pages.borrow_mut().push(artist_page);
        self.library_subpages.borrow_mut().push(SubpageType::Artist);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &str = "MellowWindow";
    type Type = super::Window;
    type ParentType = ApplicationWindow;

    fn class_init(class: &mut Self::Class) {
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

            // TODO: If possible, allow files _or_ folders
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
                            .to_owned(),
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
    }
}
impl WindowImpl for Window {
    fn close_request(&self) -> glib::Propagation {
        self.obj().save_window_size().unwrap();
        // TODO: Use the XDG desktop portal
        self.obj().set_visible(false);
        match self.main_player.is_playing() && self.settings_page.play_in_background() {
            false => glib::Propagation::Proceed,
            true => glib::Propagation::Stop,
        }
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
