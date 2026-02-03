use glib::{UserDirectory, home_dir, user_config_dir, user_special_dir};
use gtk::glib;
use std::fs::{self, DirEntry};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;
use std::{io, ptr};

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
    let _ = CONFIG_DIR.set(user_config_dir().to_str().unwrap().to_string() + "/mellow/");
    let _ = MUSIC_DIR.set(user_special_dir(UserDirectory::Music).map_or_else(
        || [home_dir().to_str().unwrap(), "/Music/"].concat(),
        |dir| dir.to_str().unwrap().to_string(),
    ));
}

/// Takes a `&Duration` and returns a nicely formatted `String`
/// for display
///
/// # Example:
/// ```rust
/// use std::time::Duration;
/// use mellow::format_duration;
///
/// let duration = Duration::from_secs(83);
/// let formatted = format_duration(&duration);
///
/// assert_eq!(formatted, "1:23".to_string());
/// ```
#[inline]
#[must_use]
pub fn format_duration(duration: &Duration) -> String {
    // TODO: Support hours
    // IDEA: Support days (for playlists, maybe `format_duration_long()`)
    let duration = duration.as_secs();
    let seconds = duration % 60;
    format!(
        "{}:{}{seconds}",
        (duration - seconds) / 60,
        if seconds < 10 { "0" } else { "" }
    )
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
                    .to_string(),
            );
            start = i + 1;
        }
    }
    match input[start..].trim().to_string() {
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

pub trait ReorderVecSafe {
    fn reorder(&mut self, index: usize, target: usize);
}
impl<T> ReorderVecSafe for Vec<T> {
    /// Moves an element of `Vec<T>` from `index` to `target`,
    /// preserving the order of other elements. Elements in
    /// between are shifted towards `index` by one.
    ///
    /// # Panics
    ///
    /// Panics if either `index` or `target` is out of bounds
    ///
    /// # Example
    /// ```rust
    /// use mellow::UnsafeReorderVec;
    ///
    /// let mut vec = vec![1, 2, 3, 4, 5];
    ///
    /// vec.reorder(1, 4);
    /// assert_eq!(vec, vec![1, 3, 4, 5, 2]);
    ///
    /// vec.reorder(4, 1);
    /// assert_eq!(vec, vec![1, 2, 3, 4, 5]);
    /// ```
    fn reorder(&mut self, index: usize, target: usize) {
        if target > index {
            for i in index..target {
                self.swap(i, i + 1);
            }
        } else {
            for i in (target + 1..=index).rev() {
                self.swap(i, i - 1);
            }
        }
    }
}

pub trait ReorderVecRaw {
    fn reorder(&mut self, index: usize, target: usize);
}
impl<T: Clone> ReorderVecRaw for Vec<T> {
    /// Moves an element of `Vec<T>` from `index` to `target`,
    /// preserving the order and shifting the elements in-between
    ///
    /// # Panics
    ///
    /// Panics if either `index` or `target` is out of bounds
    ///
    /// # Example
    /// ```rust
    /// use mellow::UnsafeReorderVec;
    ///
    /// let mut vec = vec![1, 2, 3, 4, 5];
    ///
    /// vec.reorder(1, 4);
    /// assert_eq!(vec, vec![1, 3, 4, 5, 2]);
    ///
    /// vec.reorder(4, 1);
    /// assert_eq!(vec, vec![1, 2, 3, 4, 5]);
    /// ```
    fn reorder(&mut self, index: usize, target: usize) {
        assert!(index < self.len() && target < self.len());
        let elem = self[index].clone();
        let ptr = self.as_mut_ptr();
        if index < target {
            unsafe { ptr::copy(ptr.add(index + 1), ptr.add(index), target - index) };
            unsafe { ptr::write(ptr.add(target), elem) };
        } else {
            unsafe { ptr::copy(ptr.add(target), ptr.add(target + 1), index - target) };
            unsafe { ptr::write(ptr.add(target), elem) };
        }
    }
}
