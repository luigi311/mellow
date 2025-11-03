pub mod library;
pub mod player;
pub mod ui;
pub mod window;

pub const APP_NAME: &str = "Mellow";
pub const APP_ID: &str = "com.github.userwithaname.Mellow";

use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::time::Duration;

#[inline]
#[must_use]
pub fn format_duration(duration: &Duration) -> String {
    let duration = duration.as_secs();
    let seconds = duration % 60;
    format!(
        "{}:{}{seconds}",
        (duration - seconds) / 60,
        if seconds < 10 { "0" } else { "" }
    )
}

// Taken from Rust documentation:
// https://doc.rust-lang.org/beta/std/fs/fn.read_dir.html#examples
// one possible implementation of walking a directory only visiting files
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

#[inline]
#[must_use]
pub fn approx_eq(left: f64, right: f64) -> bool {
    const TOLERANCE: f64 = 0.00005;
    f64::abs(left - right) < TOLERANCE
}
