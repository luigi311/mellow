use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::time::Duration;

pub mod excuses;
pub mod library;
pub mod player;
pub mod serializer;
pub mod tasks;
pub mod ui;

pub const APP_NAME: &str = "Mellow";
pub const APP_ID: &str = "com.github.userwithaname.Mellow";

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
/// use mellow::reorder_vec;
///
/// let mut vec = vec![1, 2, 3, 4, 5];
///
/// reorder_vec(&mut vec, 1, 4);
/// assert_eq!(vec, vec![1, 3, 4, 5, 2]);
///
/// reorder_vec(&mut vec, 4, 1);
/// assert_eq!(vec, vec![1, 2, 3, 4, 5]);
/// ```
#[inline]
pub fn reorder_vec<T>(vec: &mut [T], index: usize, target: usize) {
    if target > index {
        for i in index..target {
            vec.swap(i, i + 1);
        }
    } else {
        for i in (target + 1..=index).rev() {
            vec.swap(i, i - 1);
        }
    }
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
/// Taken from the official Rust documentation:
/// <https://doc.rust-lang.org/std/fs/fn.read_dir.html#examples>
pub fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry)) -> io::Result<()> {
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
