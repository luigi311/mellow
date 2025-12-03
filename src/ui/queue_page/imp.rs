use adw::{prelude::*, subclass::prelude::*};
use glib::clone;
use gtk::CompositeTemplate;
use gtk::{gdk, glib};
use std::cell::{OnceCell, Ref};
use std::sync::mpsc;

use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::queue_row::QueueRow;
use crate::ui::song_page::SongPage;

use crate::excuses::{EXP_INIT, EXP_RX};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/queue_page.ui")]
pub struct QueuePage {
    #[template_child]
    pub shuffle_toggle: TemplateChild<gtk::ToggleButton>,
    #[template_child]
    pub repeat_toggle: TemplateChild<gtk::ToggleButton>,

    // #[template_child]
    // song_queue_list_view: TemplateChild<gtk::ListView>,
    #[template_child]
    song_queue_scrolled_window: TemplateChild<gtk::ScrolledWindow>,
    #[template_child]
    song_queue_list_box: TemplateChild<gtk::ListBox>,

    pub song_page: OnceCell<SongPage>,
    pub navigation_view: OnceCell<adw::NavigationView>,
    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
}

#[gtk::template_callbacks]
impl QueuePage {
    #[template_callback]
    pub fn handle_set_repeat(&self, toggle_button: &gtk::ToggleButton) {
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetRepeat(toggle_button.is_active()))
            .expect(EXP_RX);
    }
    #[template_callback]
    pub fn handle_set_shuffle(&self, toggle_button: &gtk::ToggleButton) {
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetRepeat(toggle_button.is_active()))
            .expect(EXP_RX);
    }

    pub fn update_song_queue(&self, queue: Ref<'_, Box<[QueueItem]>>, index: usize) {
        // TODO: Display the list properly (model/factory/view)
        // TODO: Support reordering queue items
        // TODO: Support rating/tagging songs (AdwExpanderRow/subpage/context menu)
        // TODO: Display the entire queue
        let start = index.saturating_sub(10);
        let end = (index + 15).min(queue.len());
        self.song_queue_list_box.remove_all();
        for i in start..end {
            let queue_entry = QueueRow::default();
            match &queue[i] {
                QueueItem::Song(song) => {
                    let is_playing = i == index;

                    let mut song = song.lock().unwrap();
                    let mut info = song.info();

                    let song_info = info.basic();
                    let song_title = song_info.title.clone();
                    let album_title = song_info.album.clone();
                    let artist_name = song_info.artist.clone();

                    queue_entry.set_title(&song_title);
                    queue_entry.set_subtitle(&artist_name);
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
                            self.song_page.get().expect(EXP_INIT),
                            #[weak(rename_to=navigation)]
                            self.navigation_view.get().expect(EXP_INIT),
                            move |_| {
                                navigation.push_by_tag("info");
                                song_page.set_info(i, &song_title, &album_title, &artist_name);
                            }
                        )
                    });
                }
                QueueItem::Stopper => {
                    queue_entry.set_title("Pause");
                    queue_entry.add_css_class("heading");
                    queue_entry.add_css_class("dimmed");

                    // IDEA: Draw a pause icon in place of the album cover
                    // queue_entry.set_prefix_image();

                    // TODO: Open a page for stoppers as well
                    // TODO: Allow removing stoppers
                    // TODO: Allow reordering stoppers
                }
            }
            self.song_queue_list_box.append(&queue_entry);
        }
        let new_value = (index - start) * 54;
        self.song_queue_scrolled_window
            .vadjustment()
            .set_value(new_value as f64);

        // Garbage collection
        for (index, item) in queue.iter().enumerate() {
            if (start..end).contains(&index) {
                continue;
            }
            if let QueueItem::Song(song) = item {
                let _ = song.lock().map(|mut song| {
                    song.info().unload_detailed();
                });
            }
        }
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
