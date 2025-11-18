use adw::{self, Application, prelude::*};
use gst::{ClockTime, State};
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk::{self, glib};
use std::sync::mpsc;
use tokio::sync::mpsc as tokio_mpsc;

mod queue_row;
mod rating;
mod song_page;
mod window;

use crate::player::PlayerRequest;
use crate::player::song_queue::QueueItem;
use crate::ui::window::Window;
use crate::{APP_ID, APP_NAME};

pub enum UpdateUI {
    PlayerState(State, bool),
    PlayerTime(Option<ClockTime>),
    SongInfo,
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
    let window = Window::new(app, player_tx.clone());
    window.set_title(Some(APP_NAME));
    window.set_icon_name(Some(APP_ID));
    window.present();

    glib::spawn_future_local(async move { window.imp().event_handler(ui_rx).await });
}
