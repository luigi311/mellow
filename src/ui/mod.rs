use adw::{self, Application, prelude::*};
use gst::ClockTime;
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{self, glib};
use std::sync::mpsc;
use tokio::sync::mpsc as tokio_mpsc;

mod library_albums_page;
mod library_artists_page;
mod library_home_page;
mod library_songs_page;
mod lyrics_page;
mod main_player;
mod queue_page;
mod queue_row;
mod rating;
mod settings_page;
mod song_page;
mod window;

use crate::library::LibraryRequest;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::window::Window;
use crate::{APP_ID, APP_NAME};

pub enum UpdateUI {
    /// (playing: bool, interactive: bool)
    PlayerState(bool, bool),
    PlayerTime(Option<ClockTime>),
    SongInfo,
    SongQueue(Box<[QueueItem]>),
    QueueIndex(usize),
    Shuffle(bool),
    Repeat(bool),
    Progress(Option<f64>),
    FocusLibrary,
    FocusPlaying,
    FocusSettings,
    OpenSheet(bool),
}

pub fn init(
    app: &Application,
    library_tx: &mpsc::SyncSender<LibraryRequest>,
    player_tx: &mpsc::SyncSender<PlayerRequest>,
    ui_rx: tokio_mpsc::Receiver<UpdateUI>,
) {
    let window = Window::new(app, library_tx.clone(), player_tx.clone());
    window.set_title(Some(APP_NAME));
    window.set_icon_name(Some(APP_ID));
    window.present();

    glib::spawn_future_local(async move { window.imp().event_handler(ui_rx).await });
}
