use adw::{self, Application, prelude::*, subclass::prelude::*};
use gst::ClockTime;
use gtk::{self, glib};
use std::sync::OnceLock;
use tokio::sync::mpsc as tokio_mpsc;

mod album_object;
mod album_page;
mod album_tile;
mod library_albums_page;
mod library_artists_page;
mod library_home_page;
mod library_songs_page;
mod lyrics_page;
mod main_player;
mod queue_page;
mod queue_row;
mod queue_song_page;
mod rating;
mod settings_page;
mod window;

use crate::about::{APP_ID, APP_NAME};
use crate::library::{Albums, Artists, Songs};
use crate::player::song_queue::QueueItem;
use crate::ui::window::Window;

pub static UI_TX: OnceLock<tokio_mpsc::UnboundedSender<UpdateUI>> = OnceLock::new();
pub enum UpdateUI {
    /// (playing: bool, interactive: bool)
    PlayerState(bool, bool),
    PlayerTime(Option<ClockTime>),
    SongInfo,
    NewQueue(Box<[QueueItem]>), // TODO: QueueInsert, QueueRemove, QueueReorder, QueueSwap
    QueueIndex(usize),
    RedrawQueue,
    QueueSupbage(usize),
    Shuffle(bool),
    Repeat(bool),
    Progress(Option<f64>),

    LibraryDirs(Box<[String]>),
    LibrarySongs(Songs),
    LibraryAlbums(Albums),
    LibraryArtists(Artists),
    AlbumPage(usize),

    FocusLibrary,
    FocusPlaying,
    FocusSettings,
    OpenSheet(bool),
}

/// Starts the application and initializes `UI_TX`
#[inline]
pub fn init(app: &Application, ui_rx: tokio_mpsc::UnboundedReceiver<UpdateUI>) {
    let window = Window::new(app);
    window.set_title(Some(APP_NAME));
    window.set_icon_name(Some(APP_ID));
    window.present();

    glib::spawn_future_local(async move { window.imp().event_handler(ui_rx).await });
}
