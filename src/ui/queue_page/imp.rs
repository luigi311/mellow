use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::OnceCell;
use std::thread;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, LibraryRequest};
use crate::player::queue_item::QueueItem;
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::ui::queue_subpage::QueueSubpage;
use crate::ui::song_row::SongRow;
use crate::ui::{UI_TX, UpdateUI, fallback_song_image};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/queue_page.ui")]
pub struct QueuePage {
    #[template_child]
    pub shuffle_toggle: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pub repeat_toggle: TemplateChild<gtk::ToggleButton>,

    #[template_child]
    list_box: TemplateChild<gtk::ListBox>,
    // list_view: TemplateChild<gtk::ListView>,
    #[template_child]
    scrolled_window: TemplateChild<gtk::ScrolledWindow>,

    #[template_child]
    view_stack: TemplateChild<adw::ViewStack>,

    pub song_page: OnceCell<QueueSubpage>,
}

#[gtk::template_callbacks]
impl QueuePage {
    #[template_callback]
    pub fn handle_set_repeat(&self, toggle_button: &gtk::ToggleButton) {
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetRepeat(toggle_button.is_active()))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_set_shuffle(&self, toggle_button: &gtk::ToggleButton) {
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetShuffle(toggle_button.is_active()))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_open_library(&self) {
        UI_TX
            .get()
            .expect(EXP_INIT)
            .send(UpdateUI::FocusLibrary)
            .expect(EXP_RX);
    }

    pub fn update_song_queue(&self, queue: &[QueueItem], index: usize) {
        self.view_stack
            .set_visible_child_name(match queue.is_empty() {
                true => "queue_empty",
                false => "song_queue",
            });

        // TODO: Display the list properly (model/factory/view)
        // TODO: Support reordering queue items (needs UI)
        // TODO: Display the entire queue
        self.list_box.remove_all();
        let start = index.saturating_sub(45);
        let end = (index + 45).min(queue.len());
        let mut needs_loading = false;
        for (i, item) in queue.iter().enumerate().take(end).skip(start) {
            match item {
                QueueItem::Song(song) => {
                    let mut song = song.lock().unwrap();
                    let mut info = song.info();

                    let song_info = info.basic();
                    let is_playing = i == index;

                    let entry = SongRow::default();
                    entry.set_title(&song_info.title);
                    entry.set_subtitle(&song_info.artist);
                    if is_playing {
                        entry.add_css_class("heading");
                        entry.add_css_class("card");
                    }

                    // TODO: Cached low-res album covers
                    let artwork = info.inspect_detailed().map_or_else(
                        || {
                            needs_loading = true;
                            None
                        },
                        |info| info.artwork.as_ref(),
                    );
                    if artwork.is_some() {
                        entry.set_prefix_image(artwork);
                    } else {
                        entry.set_prefix_image(Some(&fallback_song_image()));
                    }

                    drop(song);

                    entry.connect_activated(move |_| {
                        UI_TX
                            .get()
                            .expect(EXP_INIT)
                            .send(UpdateUI::QueueSupbage(i))
                            .expect(EXP_RX);
                    });

                    self.list_box.append(&entry);
                }
                QueueItem::Stopper => {
                    let entry = SongRow::default();
                    entry.set_title("Pause");
                    entry.add_css_class("heading");
                    entry.add_css_class("dimmed");

                    // IDEA: Draw a pause icon in place of the album cover
                    // queue_entry.set_prefix_image();

                    self.list_box.append(&entry);
                }
            }
        }

        let scroll_target = (index - start) * 54;
        self.scrolled_window
            .vadjustment()
            .set_value(scroll_target as f64);

        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        if !needs_loading {
            let songs = queue.to_vec();
            library_tx
                .send(LibraryRequest::RunTask(Box::new(move || {
                    // Garbage collection
                    for (index, song) in songs.iter().enumerate() {
                        if !(start..end).contains(&index)
                            && let QueueItem::Song(song) = song
                            && let Ok(mut song) = song.try_lock()
                        {
                            song.info().unload_detailed();
                        }
                    }
                })))
                .expect(EXP_RX);

            return; // Skip loading artworks
        }

        let load_artworks_handle = thread::spawn({
            let songs = queue[start..end].to_vec();
            move || {
                println!("Loading artworks for queued songs");
                for song in songs.iter().rev() {
                    if let QueueItem::Song(song) = song {
                        let _ = song.try_lock().map(|mut song| song.info().load_detailed());
                    }
                }
                UI_TX
                    .get()
                    .expect(EXP_INIT)
                    .send(UpdateUI::RedrawQueue)
                    .expect(EXP_RX);
            }
        });
        library_tx
            .send(LibraryRequest::RunTask(Box::new(move || {
                let _ = load_artworks_handle.join();
            })))
            .expect(EXP_RX);
    }
}

#[glib::object_subclass]
impl ObjectSubclass for QueuePage {
    const NAME: &str = "MellowQueuePage";
    type Type = super::QueuePage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}
impl ObjectImpl for QueuePage {
    fn constructed(&self) {
        self.obj().update_shuffle(false);
        self.obj().update_repeat(false);
    }
}

impl WidgetImpl for QueuePage {}
impl NavigationPageImpl for QueuePage {}
