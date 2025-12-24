use adw::{prelude::*, subclass::prelude::*};
use glib::clone;
use gtk::CompositeTemplate;
use gtk::{gdk, glib};
use std::cell::OnceCell;
use std::sync::Arc;

use crate::excuses::{ACTION_ERR, EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, LibraryRequest};
use crate::player::song_queue::QueueItem;
use crate::player::{PLAYER_TX, PlayerRequest};
use crate::ui::UI_TX;
use crate::ui::queue_row::QueueRow;
use crate::ui::queue_song_page::QueueSongPage;

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

    pub song_page: OnceCell<QueueSongPage>,
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

    pub fn update_song_queue(&self, queue: &[QueueItem], index: usize) {
        // TODO: Display the list properly (model/factory/view)
        // TODO: Support reordering queue items
        // TODO: Display the entire queue
        self.list_box.remove_all();
        let start = index.saturating_sub(10);
        let end = (index + 15).min(queue.len());
        for (i, item) in queue.iter().enumerate() {
            match item {
                item if !(start..end).contains(&i) => {
                    // Garbage collection
                    if i < end + 15 && i > start.saturating_sub(10) {
                        // WORKAROUND: Load more items than needed in the
                        // background (and keep them loaded), so the UI
                        // doesn't stutter when switching songs
                        if let QueueItem::Song(song) = item {
                            LIBRARY_TX
                                .get()
                                .expect(EXP_INIT)
                                .send(LibraryRequest::RunTask(Box::new({
                                    let song = Arc::clone(song);
                                    move || song.lock().unwrap().info().load_detailed()
                                })))
                                .expect(EXP_RX);
                        }
                        continue;
                    }
                    if let QueueItem::Song(song) = item {
                        song.lock().unwrap().info().unload_detailed();
                    }
                }
                QueueItem::Song(song) => {
                    let mut song = song.lock().unwrap();
                    let mut info = song.info();

                    let song_info = info.basic();
                    let song_title = song_info.title.clone();
                    let album_title = song_info.album.clone();
                    let artist_name = song_info.artist.clone();
                    let is_playing = i == index;

                    let entry = QueueRow::default();
                    entry.set_title(&song_title);
                    entry.set_subtitle(&artist_name);
                    if is_playing {
                        entry.add_css_class("heading");
                        entry.add_css_class("card");
                    }

                    // TODO: Cached low-res album covers
                    if let Some(detailed_info) = info.inspect_detailed()
                        && let Some(artwork) = detailed_info.artwork.as_ref()
                    {
                        entry.set_prefix_image(artwork);
                    } else {
                        entry.set_prefix_image(&gdk::Paintable::new_empty(1, 1));
                    }

                    entry.connect_activated({
                        clone!(
                            #[weak(rename_to=song_page)]
                            self.song_page.get().expect(EXP_INIT),
                            move |_| {
                                song_page
                                    .activate_action(
                                        "ui.playing_nav_push",
                                        Some(&"info".to_variant()),
                                    )
                                    .expect(ACTION_ERR);
                                song_page.set_info(i, &song_title, &album_title, &artist_name);
                            }
                        )
                    });

                    self.list_box.append(&entry);
                }
                QueueItem::Stopper => {
                    let entry = QueueRow::default();
                    entry.set_title("Pause");
                    entry.add_css_class("heading");
                    entry.add_css_class("dimmed");

                    // IDEA: Draw a pause icon in place of the album cover
                    // queue_entry.set_prefix_image();

                    // TODO: Open a page for stoppers as well
                    // TODO: Allow removing stoppers
                    // TODO: Allow reordering stoppers

                    self.list_box.append(&entry);
                }
            }
        }
        LIBRARY_TX
            .get()
            .expect(EXP_INIT)
            .send(LibraryRequest::RunTask(Box::new({
                let songs = queue[start..end].to_vec();
                move || {
                    let mut updated = false;
                    for song in songs.iter().rev() {
                        match song {
                            QueueItem::Song(song) => {
                                let _ = song.try_lock().map(|mut song| {
                                    let _ = song.info().detailed_and(|| updated = true);
                                });
                            }
                            QueueItem::Stopper => (),
                        }
                    }
                    if updated {
                        UI_TX
                            .get()
                            .expect(EXP_INIT)
                            .send(crate::ui::UpdateUI::QueueIndex(index))
                            .expect(EXP_RX);
                    }
                }
            })))
            .expect(EXP_RX);

        let scroll_target = (index - start) * 54;
        self.scrolled_window
            .vadjustment()
            .set_value(scroll_target as f64);
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
