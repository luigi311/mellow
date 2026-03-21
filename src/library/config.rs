use core::str::Chars;
use gio::prelude::FileExt;
use gtk::gio;
use std::fs;

use crate::CONFIG_DIR;
use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, LibraryRequest};
use crate::ui::{UI_TX, UpdateUI};

pub const FILE_SUPPORT: &[&str] = &[
    "flac", "m4a", "mp3", "aac", "ac3", "wav",
    // TODO: Ensure all listed formats work
    // Untested:
    "ape", "mpc", "ogg",
];

#[derive(Clone)]
pub struct LibraryConfig {
    pub directories: Vec<String>,
    pub dir: String,
    uri_opt: usize,
}

impl LibraryConfig {
    /// Creates a new instance of `LibraryConfig` and assigns the provided `directories`
    ///
    /// # Panics
    /// The function panics if the `CONFIG_DIR` global variable is uninitialized
    #[inline]
    pub fn new(directories: Vec<String>) -> Self {
        let mut config = LibraryConfig {
            directories,
            dir: CONFIG_DIR.get().expect(EXP_INIT).clone(),
            uri_opt: 0,
        };
        config.update_trim_uri();
        config
    }

    /// Replaces the configured directories with `dirs`
    pub fn set_libraries(&mut self, dirs: &[String]) {
        self.directories = dirs.into();
        self.directories.sort();
        println!(
            "Library directories updated\nLibraries: {:?}",
            self.directories
        );
        self.update_library();
        self.update_trim_uri();
    }

    /// Adds `dir` to the configured directories
    pub fn add_library(&mut self, dir: String) {
        if self.directories.contains(&dir) || dir.is_empty() {
            return;
        }
        self.directories.push(dir);
        self.directories.sort();
        println!("Added a new library\nLibraries: {:?}", self.directories);
        self.update_library();
        self.update_trim_uri();
    }

    /// Replaces configured directory at `index` with `dir`
    pub fn edit_library(&mut self, index: usize, dir: String) {
        if self.directories.contains(&dir) {
            return self.remove_library(index);
        }
        self.directories[index] = dir;
        self.directories.sort();
        println!("Edited a library\nLibraries: {:?}", self.directories);
        self.update_library();
        self.update_trim_uri();
    }

    /// Removes the configured directory at `index`
    pub fn remove_library(&mut self, index: usize) {
        let removed_dir = self.directories.remove(index);
        println!("Removed a library\nLibraries: {:?}", self.directories);

        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let _ = library_tx.send(LibraryRequest::CancelRebuild);
        let _ = library_tx.send(LibraryRequest::RegisterUndoDirectory(removed_dir.clone()));

        let ui_tx = UI_TX.get().expect(EXP_INIT);
        let _ = ui_tx.send(UpdateUI::Notification(
            format!("Removed a library directory: {removed_dir}"),
            Some(Box::new(move || {
                (LIBRARY_TX.get().expect(EXP_INIT))
                    .send(LibraryRequest::UndoRemovedDirectory(removed_dir.to_owned()))
                    .expect(EXP_RX);
            })),
        ));
        let _ = ui_tx.send(UpdateUI::SetLibraryDirs(self.directories.clone().into()));

        let _ = library_tx.send(LibraryRequest::Rebuild);

        // self.update_library();
        self.update_trim_uri();
    }

    /// Requests a library rebuild and updates the directory list in the UI
    fn update_library(&self) {
        let ui_tx = UI_TX.get().expect(EXP_INIT);
        let _ = ui_tx.send(UpdateUI::SetLibraryDirs(self.directories.clone().into()));

        let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
        let _ = library_tx.send(LibraryRequest::CancelRebuild);
        let _ = library_tx.send(LibraryRequest::Rebuild);
    }

    /// Updates the `uri_opt` property, used to optimize song index lookups
    ///
    /// For example, for `["file:///home/Music", "file:///home/Other"]`,
    /// the common part is `"file:///home/"`, so `uri_opt` becomes 13
    ///
    /// Note: If spaces or special characters are common between directories,
    /// the assigned value may be shorter than necessary
    pub fn update_trim_uri(&mut self) {
        match self.directories.len() {
            0 => return self.uri_opt = 0,
            1 => return self.uri_opt = self.directories[0].len() + "file://".len(),
            _ => self.uri_opt = 0,
        }

        let mut dirs: Vec<Chars> = self.directories.iter().map(|dir| dir.chars()).collect();
        'counter: loop {
            let chars: Vec<Option<char>> = dirs.iter_mut().map(|c| c.next()).collect();
            for i in 1..chars.len() {
                // SAFETY: Range ensures `i` is less than `chars.len()`
                let cur = unsafe { chars.get_unchecked(i) };
                // SAFETY: Range ensures `i` is at least 1
                let last = unsafe { chars.get_unchecked(i - 1) };

                if cur != last || cur.is_none() {
                    break 'counter;
                }
            }
            // SAFETY: `get_unchecked(0)`: `chars` cannot be empty due to early return
            // SAFETY: `unwrap_unchecked()`: outer loop exits if any char is `None`
            self.uri_opt += unsafe { chars.get_unchecked(0).unwrap_unchecked().len_utf8() };
        }
        self.uri_opt += "file://".len();
    }

    /// Updates the `uri_opt` property, used to optimize song index lookups
    ///
    /// For example, for `["file:///home/Music", "file:///home/Other"]`,
    /// the common part is `"file:///home/"`, so `uri_opt` becomes 13
    ///
    /// This is an older version of the function; it might be worth
    /// benchmarking to see which implementation is faster
    pub fn update_trim_uri_old(&mut self) {
        if self.directories.is_empty() {
            return;
        }
        self.uri_opt = usize::MAX;
        let mut last_dir = self.directories[0].chars();
        for dir in &self.directories {
            let mut new_chars = dir.chars().take(self.uri_opt);
            let mut old_chars = last_dir.clone().take(self.uri_opt);
            last_dir = dir.chars();
            let mut len = 0;
            while let (Some(new), Some(old)) = (new_chars.next(), old_chars.next()) {
                if old != new {
                    break;
                }
                len += new.len_utf8();
            }
            self.uri_opt = self
                .uri_opt
                .min(gio::File::for_path(&dir[0..len]).uri().len());
        }
    }

    /// Returns the length of characters all configured directories' URIs
    /// have in common (the length until the first differing character)
    ///
    /// For example, for `["file:///home/Music", "file:///home/Other"]`,
    /// the common part is `"file:///home/"`, and the returned value is 13
    #[inline]
    #[must_use]
    pub const fn uri_opt(&self) -> usize {
        self.uri_opt
    }

    /// Creates the config directory if it does not exist yet
    ///
    /// # Panics
    /// Panics if directory creation fails
    #[inline]
    pub fn create_config_dir() {
        fs::create_dir_all(CONFIG_DIR.get().expect(EXP_INIT))
            .expect("Could not create the config directory");
    }
}
