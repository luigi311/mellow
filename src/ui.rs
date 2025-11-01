use adw::{self, Application, prelude::*};
use gst::{ClockTime, State};
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{self, glib};
use std::sync::{Arc, mpsc};
use tokio::sync::mpsc as tokio_mpsc;

use crate::library::SongInfo;
use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::window::Window;
use crate::{APP_ID, APP_NAME};

pub enum UpdateUI {
    PlayerState(State, bool),
    PlayerTime(Option<ClockTime>),
    SongInfo(Option<Arc<SongInfo>>),
    SongQueue(Box<[QueueItem]>),
    QueueIndex(usize),
    Shuffle(bool),
    Repeat(bool),
    Progress(Option<f64>),
    OpenLibrary,
}

pub fn build(
    app: &Application,
    player_tx: &mpsc::SyncSender<PlayerRequest>,
    ui_rx: tokio_mpsc::Receiver<UpdateUI>,
) {
    let window = Window::new(app);
    window.set_title(Some(APP_NAME));
    window.set_icon_name(Some(APP_ID));
    window.register_player_tx(player_tx.clone());
    window.present();

    glib::spawn_future_local(async move { window.imp().event_handler(ui_rx).await });
}
