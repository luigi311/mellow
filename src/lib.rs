#![deny(clippy::missing_safety_doc, clippy::undocumented_unsafe_blocks)]
#![warn(
    clippy::clear_with_drain,
    clippy::deref_by_slicing,
    clippy::doc_markdown,
    clippy::fallible_impl_from,
    clippy::missing_errors_doc,
    // clippy::missing_panics_doc,
    clippy::mixed_read_write_in_expression,
    clippy::must_use_candidate,
    clippy::needless_collect,
    clippy::needless_for_each,
    clippy::needless_pass_by_ref_mut,
    // clippy::needless_pass_by_value,
    clippy::semicolon_if_nothing_returned,
    clippy::single_option_map,
    // clippy::std_instead_of_core,
    clippy::str_to_string,
    clippy::unnecessary_safety_comment,
    clippy::unnecessary_safety_doc,
    clippy::unnecessary_semicolon,
    unused_unsafe
)]
#![allow(clippy::match_bool)]

use glib::{UserDirectory, home_dir, user_cache_dir, user_config_dir, user_special_dir};
use gtk::glib;
use std::{sync::OnceLock, time::Duration};

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
pub fn init_globals() {
    let _ = CACHE_DIR.set([user_cache_dir().to_str().unwrap(), "/mellow/"].concat());
    let _ = CONFIG_DIR.set([user_config_dir().to_str().unwrap(), "/mellow/"].concat());
    let _ = MUSIC_DIR.set(user_special_dir(UserDirectory::Music).map_or_else(
        || [home_dir().to_str().unwrap(), "/Music/"].concat(),
        |dir| dir.to_str().unwrap().to_owned(),
    ));
}
