#![deny(clippy::missing_safety_doc, clippy::undocumented_unsafe_blocks)]
#![warn(
    clippy::clear_with_drain,
    clippy::deref_by_slicing,
    clippy::doc_markdown,
    clippy::fallible_impl_from,
    clippy::missing_const_for_fn,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::mixed_read_write_in_expression,
    clippy::must_use_candidate,
    clippy::needless_collect,
    clippy::needless_for_each,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    clippy::semicolon_if_nothing_returned,
    clippy::single_option_map,
    // clippy::std_instead_of_core,
    clippy::str_to_string,
    clippy::unnecessary_semicolon,
    unused_unsafe
)]
#![allow(clippy::match_bool)]

use glib::{UserDirectory, home_dir, user_cache_dir, user_config_dir, user_special_dir};
use gtk::glib;
use std::sync::{OnceLock, mpsc};
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::library::{LibraryRequest, init_library_tx};
use crate::player::{PlayerRequest, init_player_tx};
use crate::ui::{UpdateUI, init_ui_tx};

pub mod about;
pub mod excuses;
pub mod library;
pub mod player;
pub mod ui;
pub mod util;

pub static CACHE_DIR: OnceLock<String> = OnceLock::new();
pub static CONFIG_DIR: OnceLock<String> = OnceLock::new();
pub static MUSIC_DIR: OnceLock<String> = OnceLock::new();

pub const UI_TIMEOUT: Duration = Duration::from_millis(1000 / 60);

/// Initializes the `CONFIG_DIR` and `MUSIC_DIR` global variables
/// (does nothing if already initialized)
///
/// # Panics
/// The function panics if user directories are not valid UTF-8
#[inline]
pub fn init_globals() {
    let _ = CACHE_DIR.set([user_cache_dir().to_str().unwrap(), "/mellow/"].concat());
    let _ = CONFIG_DIR.set([user_config_dir().to_str().unwrap(), "/mellow/"].concat());
    let _ = MUSIC_DIR.set(user_special_dir(UserDirectory::Music).map_or_else(
        || [home_dir().to_str().unwrap(), "/Music/"].concat(),
        |dir| dir.to_str().unwrap().to_owned(),
    ));
}

/// Initializes the channel senders (`ui_tx`/`player_tx`/`library_tx`),
/// and returns their receivers, which should be forwarded to the
/// respective `init` functions.
#[inline]
#[must_use = "Caution: Channel receivers must be used, otherwise the channels will close"]
pub fn init_channels() -> (
    tokio_mpsc::UnboundedReceiver<UpdateUI>,
    mpsc::Receiver<PlayerRequest>,
    mpsc::Receiver<LibraryRequest>,
) {
    let (ui_tx, ui_rx) = tokio_mpsc::unbounded_channel::<UpdateUI>();
    let (player_tx, player_rx) = mpsc::channel::<PlayerRequest>();
    let (library_tx, library_rx) = mpsc::channel::<LibraryRequest>();

    init_ui_tx(ui_tx);
    init_player_tx(player_tx);
    init_library_tx(library_tx);

    (ui_rx, player_rx, library_rx)
}

/// Returns the music directory path
///
/// # Safety
/// Causes undefined behavior if called before `init_globals`
#[inline]
#[must_use]
pub fn music_dir() -> &'static String {
    // SAFETY: `init_globals` is called in `main`, before the application starts
    unsafe { MUSIC_DIR.get().unwrap_unchecked() }
}
/// Returns the cache directory path
///
/// # Safety
/// Causes undefined behavior if called before `init_globals`
#[inline]
#[must_use]
pub fn cache_dir() -> &'static String {
    // SAFETY: `init_globals` is called in `main`, before the application starts
    unsafe { CACHE_DIR.get().unwrap_unchecked() }
}
/// Returns the configuration directory path
///
/// # Safety
/// Causes undefined behavior if called before `init_globals`
#[inline]
#[must_use]
pub fn config_dir() -> &'static String {
    // SAFETY: `init_globals` is called in `main`, before the application starts
    unsafe { CONFIG_DIR.get().unwrap_unchecked() }
}
/// Returns the `songs` file path, which contains the serialized info of library songs
///
/// # Safety
/// Causes undefined behavior if called before `init_globals`
#[inline]
#[must_use]
pub fn songs_file() -> String {
    [config_dir(), "songs"].concat()
}
/// Returns the `queue` file path, used to restore the previous queue
///
/// # Safety
/// Causes undefined behavior if called before `init_globals`
#[inline]
#[must_use]
pub fn queue_file() -> String {
    [config_dir(), "queue"].concat()
}
/// Returns the `shuffled_queue` file path, used to remember
/// the previous shuffled songs order in the restored queue
///
/// # Safety
/// Causes undefined behavior if called before `init_globals`
#[inline]
#[must_use]
pub fn shuffled_queue_file() -> String {
    [config_dir(), "shuffled_queue"].concat()
}
