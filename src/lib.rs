#![deny(unused_unsafe, clippy::undocumented_unsafe_blocks)]
#![warn(
    clippy::clear_with_drain,
    clippy::deref_by_slicing,
    clippy::doc_markdown,
    clippy::fallible_impl_from,
    clippy::mixed_read_write_in_expression,
    clippy::must_use_candidate,
    clippy::needless_collect,
    clippy::needless_for_each,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_pass_by_value,
    clippy::single_option_map,
    clippy::str_to_string
)]

use glib::{UserDirectory, home_dir, user_config_dir, user_special_dir};
use gtk::glib;
use std::fs::{self, DirEntry};
use std::path::Path;
use std::sync::OnceLock;
use std::{io, mem, ptr};

pub mod about;
pub mod excuses;
pub mod library;
pub mod player;
pub mod serializer;
pub mod tasks;
pub mod ui;

pub static CONFIG_DIR: OnceLock<String> = OnceLock::new();
pub static MUSIC_DIR: OnceLock<String> = OnceLock::new();

/// Initializes the `CONFIG_DIR` and `MUSIC_DIR` global variables
/// (does nothing if already initialized)
///
/// # Panics
/// The function panics if user directories are not valid UTF-8
pub fn init_globals() {
    let _ = CONFIG_DIR.set(user_config_dir().to_str().unwrap().to_owned() + "/mellow/");
    let _ = MUSIC_DIR.set(user_special_dir(UserDirectory::Music).map_or_else(
        || [home_dir().to_str().unwrap(), "/Music/"].concat(),
        |dir| dir.to_str().unwrap().to_owned(),
    ));
}

/// Takes a duration in seconds and returns a nicely formatted
/// `String` for display
///
/// # Example:
/// ```rust
/// use std::time::Duration;
/// use mellow::format_duration;
///
/// assert_eq!(format_duration(83), "1:23");
/// assert_eq!(format_duration(60 * 60 + 83), "1:01:23");
/// ```
#[inline]
#[must_use]
pub fn format_duration(seconds_total: u64) -> String {
    let seconds = seconds_total % 60;
    if seconds_total < 60 * 60 {
        format!("{}:{seconds:02}", (seconds_total - seconds) / 60)
    } else {
        let minutes_total = (seconds_total - seconds) / 60;
        let minutes = minutes_total % 60;
        format!(
            "{}:{minutes:02}:{seconds:02}",
            (minutes_total - minutes) / 60,
        )
    }
}
/// Takes a duration in milliseconds and returns a nicely
/// formatted `String` for display
///
/// # Example:
/// ```rust
/// use std::time::Duration;
/// use mellow::format_duration_ms;
///
/// assert_eq!(format_duration_ms(83000), "1:23");
/// ```
#[inline]
#[must_use]
pub fn format_duration_ms(milliseconds_total: u64) -> String {
    format_duration(milliseconds_total / 1000)
}
#[inline]
#[must_use]
pub fn format_duration_minutes(minutes_total: u64) -> String {
    todo!("TODO: Format long times (minutes, hours, days)")
}

/// Returns a value between `left` and `right` at point `mid`
/// The `mid` point maps values between 0 and 1 such that 0 is `left`
/// and 1 is `right`. Values outside the 0 to 1 range are also allowed
///
/// # Example
/// ```rust
/// use mellow::lerp;
///
/// assert_eq!(lerp(5.0, 10.0, 0.0), 5.0);
/// assert_eq!(lerp(5.0, 10.0, 1.0), 10.0);
/// assert_eq!(lerp(5.0, 10.0, 0.5), 7.5);
/// assert_eq!(lerp(5.0, 10.0, 2.0), 15.0);
/// assert_eq!(lerp(5.0, 10.0, -1.0), 0.0);
/// ```
#[must_use]
pub fn lerp(left: f64, right: f64, mid: f64) -> f64 {
    (right - left).mul_add(mid, left)
}

/// Checks if two float numbers are similar
///
/// # Example
/// ```rust
/// use mellow::approx_eq;
///
/// assert!(approx_eq(0.9995, 1.0));
/// assert!(approx_eq(1.0005, 1.0));
/// assert!(!approx_eq(0.9994, 1.0));
/// ```
#[inline]
#[must_use]
pub fn approx_eq(left: f64, right: f64) -> bool {
    const TOLERANCE: f64 = 0.0005;
    (left - right).abs() < TOLERANCE
}

/// Splits the input string at every occurence of a character,
/// and supports `\` as the un-escape character in the input
///
/// Whitespaces around each split are trimmed, and escape
/// characters are not included in the output result
///
/// Note that splitting by `\` is not possible
///
/// # Example:
/// ```rust
/// use mellow::unescaped_split;
///
/// assert_eq!(
///     unescaped_split(r"Testing, testing\, one two, three", ',').as_ref(),
///     vec!["Testing", "testing, one two", "three"]
/// );
/// assert_eq!(
///     unescaped_split(r"Testing? testing\? one two? three", '?').as_ref(),
///     vec!["Testing", "testing? one two", "three"]
/// );
/// ```
#[inline]
#[must_use]
pub fn unescaped_split(input: &str, character: char) -> Vec<String> {
    let chars: Vec<u8> = input.bytes().collect();
    let mut start = 0;
    let mut output = Vec::new();
    for i in 0..chars.len() {
        if chars[i] == character as u8 {
            if i > 0 && chars[i - 1] == b'\\' && (i < 2 || chars[i - 2] != b'\\') {
                continue;
            }
            output.push(
                input[start..i]
                    .replace(&format!("\\{character}"), &character.to_string())
                    .trim()
                    .to_owned(),
            );
            start = i + 1;
        }
    }
    match input[start..].trim().to_owned() {
        last if !last.is_empty() => output.push(last),
        _ => (),
    }
    output
}

/// Runs a closure for every file found within `dir` (recursive)
///
/// Adapted from the official Rust documentation:
/// <https://doc.rust-lang.org/std/fs/fn.read_dir.html#examples>
///
/// # Errors
/// The function errors if a file or directory cannot be read
pub fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&DirEntry)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}

pub trait ReorderVecExt {
    fn reorder(&mut self, index: usize, target: usize);
}
impl<T> ReorderVecExt for Vec<T> {
    /// Moves an element of `Vec<T>` from index `from` to `to`,
    /// preserving the order and shifting the elements in-between
    ///
    /// # Panics
    /// - If either `from` or `to` is out of bounds
    /// - If type `T` is zero-sized
    ///
    /// # Example
    /// ```rust
    /// use mellow::ReorderVecExt;
    ///
    /// let mut numbers = vec![1, 2, 3, 4, 5];
    ///
    /// numbers.reorder(1, 4);
    /// assert_eq!(numbers, [1, 3, 4, 5, 2]);
    ///
    /// numbers.reorder(4, 1);
    /// assert_eq!(numbers, [1, 2, 3, 4, 5]);
    ///
    /// let mut strings =  vec![
    ///     "a".to_owned(),
    ///     "b".to_owned(),
    ///     "much longer string to test if everything still works regardless".to_owned(),
    ///     "c".to_owned(),
    /// ];
    ///
    /// strings.reorder(2, 1);
    /// assert_eq!(
    ///     strings,
    ///     [
    ///         "a",
    ///         "much longer string to test if everything still works regardless",
    ///         "b",
    ///         "c",
    ///     ]
    /// );
    /// ```
    ///
    /// Reference counted types behave as expected:
    /// ```rust
    /// use mellow::ReorderVecExt;
    /// use std::rc::Rc;
    ///
    /// let mut rcs = vec![Rc::new(1), Rc::new(2)];
    ///
    /// rcs.reorder(0, 1);
    /// assert_eq!(rcs, [2.into(), 1.into()]);
    /// assert_eq!(Rc::strong_count(&rcs[0]), 1);
    /// assert_eq!(Rc::strong_count(&rcs[1]), 1);
    /// ```
    fn reorder(&mut self, from: usize, to: usize) {
        assert!(mem::size_of::<T>() != 0, "Zero-sized types are unsupported");
        assert!(from < self.len() && to < self.len());

        let ptr = self.as_mut_ptr();
        // SAFETY: Assert at the top ensures `from` is within bounds
        let old = unsafe { ptr::read(ptr.add(from)) };

        if from < to {
            // Copy everything after `from` up to and including `to` one to the left:
            // [++f---t++] => [++---tt++]

            // SAFETY: Because `from` and `to` are checked to be within bounds
            // and `from` < `to`, the following cannot exceed the allocation
            unsafe { ptr::copy(ptr.add(from + 1), ptr.add(from), to - from) };

            // Then overwrite the duplicate item using the original `from` value:
            // [++---tt++] => [++---tf++]
        } else {
            // Copy everything before `to` up to and including `from` one to the right:
            // [++t---f++] => [++tt---++]

            // SAFETY: Because `from` and `to` are checked to be within bounds
            // and `from` >= `to`, the following cannot exceed the allocation
            unsafe { ptr::copy(ptr.add(to), ptr.add(to + 1), from - to) };

            // Then overwrite the duplicate item using the original `from` value:
            // [++---tt++] => [++---tf++]
        }

        // Overwrite the position at `to` using the original value of `from`
        // SAFETY: Assert at the top ensures `to` is within bounds
        unsafe { ptr::write(ptr.add(to), old) };
    }
}
