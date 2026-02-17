use adw::{self, Application, prelude::*, subclass::prelude::*};
use gst::ClockTime;
use gtk::{self, gdk, gio, glib};
use std::sync::OnceLock;
use tokio::sync::mpsc as tokio_mpsc;

mod actions;
mod album_object;
mod album_page;
mod album_row;
mod albums_page;
mod artist_object;
mod artist_page;
mod artists_page;
mod item_row;
mod item_tile;
mod library_page;
mod lyrics_page;
mod main_player;
mod queue_item_object;
mod queue_page;
mod queue_subpage;
mod rating;
mod song_object;
mod song_page;
mod song_row;
mod songs_page;
mod window;

pub mod settings_page;

use crate::about;
use crate::library::album::SharedAlbum;
use crate::library::artist::SharedArtist;
use crate::library::song::SharedSong;
use crate::library::{Albums, Artists, Songs, ToQueue};
use crate::player::queue_item::QueueItem;
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

    LibrarySongLoaded(usize),
    LibraryAlbumLoaded(usize),
    LibraryArtistLoaded(usize),
    QueueSongLoaded(usize),

    ArtistPageByIndex(usize),
    ArtistPage(SharedArtist),
    AlbumPageByIndex(usize),
    AlbumPage(SharedAlbum),
    SongPageByIndex(usize),
    // Maybe `dyn Fn() -> Vec<QueueItem>` would be more useful?
    // Or `Vec<QueueItem>` directly, which would also remove the
    // need for the second field
    SongPage(Box<(usize, SharedSong, Box<dyn ToQueue + Send>)>),

    FocusLibrary,
    FocusPlaying,
    FocusSettings,
    OpenSheet(bool),

    Shutdown,
}

/// Starts the application and initializes `UI_TX`
#[inline]
pub fn init(
    app: &Application,
    settings: gio::Settings,
    ui_rx: tokio_mpsc::UnboundedReceiver<UpdateUI>,
) {
    let window = Window::new(app, settings);
    window.set_title(Some(about::app_name()));
    window.set_icon_name(Some(about::app_id()));
    window.present();

    glib::spawn_future_local(async move { window.imp().event_handler(ui_rx).await });
}

// IDEA: The fallback images could be cached somehow
// (might be tricky since `gdk::Paintable` cannot be const)

// Returns a fallback image intended for artists with missing artwork
#[must_use]
pub fn fallback_artist_image() -> gdk::Paintable {
    // TODO: Fallback image for albums (maybe a symbolic disc icon?)
    gdk::Paintable::new_empty(1, 1)
}

// Returns a fallback image intended for albums with missing artwork
#[must_use]
pub fn fallback_album_image() -> gdk::Paintable {
    // TODO: Fallback image for albums (maybe a symbolic disc icon?)
    gdk::Paintable::new_empty(1, 1)
}

// Returns a fallback image intended for songs with missing album covers
#[must_use]
pub fn fallback_song_image() -> gdk::Paintable {
    // TODO: Fallback image for songs (maybe a symbolic note icon?)
    gdk::Paintable::new_empty(1, 1)
}
