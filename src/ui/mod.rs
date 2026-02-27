use core::cell::Cell;
use gtk::gdk;
use std::sync::OnceLock;
use tokio::sync::mpsc as tokio_mpsc;

mod actions;
mod album_object;
mod album_page;
mod albums_page;
mod application;
mod artist_object;
mod artist_page;
mod artists_page;
mod item_row;
mod item_tile;
mod library_page;
mod list_row;
mod lyrics_page;
mod main_player;
mod queue_item_object;
mod queue_page;
mod queue_subpage;
mod rating;
mod settings_page;
mod song_object;
mod song_page;
mod songs_page;
mod window;

pub use album_object::{AlbumData, AlbumObject, AlbumOrdering};
pub use album_page::AlbumPage;
pub use albums_page::AlbumsPage;
pub use application::Application;
pub use artist_object::{ArtistData, ArtistObject, ArtistOrdering};
pub use artist_page::ArtistPage;
pub use artists_page::ArtistsPage;
pub use item_row::ItemRow;
pub use item_tile::ItemTile;
pub use library_page::{LibraryPage, SubpageType};
pub use list_row::ListRow;
pub use lyrics_page::LyricsPage;
pub use main_player::MainPlayer;
pub use queue_item_object::{QueueItemData, QueueItemObject};
pub use queue_page::QueuePage;
pub use queue_subpage::QueueSubpage;
pub use rating::Rating;
pub use settings_page::{SettingsPage, StartupQueueChoice};
pub use song_object::{SongData, SongObject, SongOrdering};
pub use song_page::SongPage;
pub use songs_page::SongsPage;
pub use window::Window;

use crate::library::{Albums, Artists, Songs, ToQueue};
use crate::library::{SharedAlbum, SharedArtist, SharedSong};
use crate::player::QueueItem;

pub static UI_TX: OnceLock<tokio_mpsc::UnboundedSender<UpdateUI>> = OnceLock::new();
pub enum UpdateUI {
    /// (playing: `bool`, interactive: `bool`)
    PlayerState(bool, bool),
    /// Current song time in milliseconds
    PlayerTime(Option<u64>),
    /// Prompts the UI to refresh the song information
    SongInfo,
    /// Replaces the UI song queue with a new one
    SetQueue(Box<[QueueItem]>),
    /// Updates the playing song index and redraws the queue
    SetQueueIndex(usize),
    /// Opens the subpage for the queue song at the given index
    OpenQueueSubpage(usize),
    /// Closes the subpage if it is open
    CloseQueueSubpage,
    /// Informs the UI of the new shuffle mode (so icons can be updated)
    Shuffle(bool),
    /// Informs the UI of the new repeat mode (so icons can be updated)
    Repeat(bool),

    /// Updates the directory list on the settings page
    SetLibraryDirs(Box<[String]>),
    /// Updates the library songs
    SetLibrarySongs(Songs),
    /// Updates the library albums
    SetLibraryAlbums(Albums),
    /// Updates the library artists
    SetLibraryArtists(Artists),

    /// Prompts the library UI to assign the now-loaded song artwork for the item at index
    LibrarySongLoaded(usize),
    /// Prompts the library UI to assign the now-loaded album artwork for the item at index
    LibraryAlbumLoaded(usize),
    /// Prompts the library UI to assign the now-loaded artist artwork for the item at index
    LibraryArtistLoaded(usize),
    /// Prompts the queue UI to assign the now-loaded song artwork for the item at index
    QueueSongLoaded(usize),

    /// Opens the library song page for the item at the given index
    SongPageByIndex(usize),
    // Maybe `dyn Fn() -> Vec<QueueItem>` would be more useful?
    // Or `Vec<QueueItem>` directly, which would also remove the
    // need for the second field
    /// Opens a song page, with the following arguments:
    /// (index: `usize`, song: `SharedSong`, a closure returning the queue for starting playback)
    SongPage(Box<(usize, SharedSong, Box<dyn ToQueue + Send>)>),
    /// Opens an album page using a `SharedAlbum`
    AlbumPage(SharedAlbum),
    /// Opens an album page using a `SharedArtist`
    ArtistPage(SharedArtist),

    /// Focuses the 'Library' tab
    FocusLibrary,
    /// Focuses the 'Playing' tab
    FocusPlaying,
    /// Focuses the 'Settings' tab
    FocusSettings,
    /// Opens or closes the bottom sheet overlay
    OpenSheet(bool),

    /// Runs a `gio` action
    RunAction(&'static str),
    /// Shows a progress bar with the specified progress value, or hides it
    Progress(Option<f64>),

    /// Causes the channel to ignore any further requests (but does not close it)
    Shutdown,
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

#[derive(Clone, Copy)]
pub struct SortConfig<O: 'static> {
    pub ordering: &'static Cell<O>,
    pub reversed: &'static Cell<bool>,
}
impl<O> SortConfig<O> {
    /// Constructs a new instance of `SortConfig`
    ///
    /// Note: Once constructed, the data will remain
    /// in memory for the duration of the program
    #[inline]
    pub fn new(ordering: O, reversed: bool) -> SortConfig<O> {
        SortConfig {
            ordering: Box::leak(Box::new(Cell::new(ordering))),
            reversed: Box::leak(Box::new(Cell::new(reversed))),
        }
    }
}
