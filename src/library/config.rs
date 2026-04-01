use core::str::Chars;
use gio::prelude::FileExt;
use gtk::{gio, glib};
use std::fs;

use crate::config_dir;
use crate::library::{LibraryRequest, library_tx};
use crate::ui::{UpdateUI, ui_tx};

pub const FILE_SUPPORT: &[&str] = &[
    "flac", "m4a", "mp3", "aac", "ac3", "wav",
    // TODO: Ensure all listed formats work
    // Untested:
    "ape", "mpc", "ogg",
];

#[derive(Clone)]
pub struct LibraryConfig {
    pub directories: Vec<String>,
    pub directory_uris: Vec<glib::GString>,
    uri_opt: usize,
}

impl LibraryConfig {
    /// Creates a new instance of `LibraryConfig` and assigns the provided `directories`
    ///
    /// # Panics
    /// The function panics if the `CONFIG_DIR` global variable is uninitialized
    #[inline]
    #[must_use]
    pub fn new(directories: Vec<String>) -> Self {
        let mut config = LibraryConfig {
            directories,
            directory_uris: Vec::new(),
            uri_opt: 0,
        };
        config.update_uris();
        config.update_trim_uri();
        config
    }

    /// Replaces the configured directories with `dirs`
    pub fn set_libraries(&mut self, dirs: &[String]) {
        self.directories = dirs.into();
        self.directories.sort();
        self.update_uris();
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
        self.update_uris();
        println!("Added a new library\nLibraries: {:?}", self.directories);
        self.update_library();
        self.update_trim_uri();
    }

    /// Removes the configured directory at `index`
    pub fn remove_library(&mut self, index: usize) {
        let library_tx = library_tx();
        let _ = library_tx.send(LibraryRequest::CancelRebuild);

        let removed_dir = self.directories.remove(index);
        self.directory_uris.remove(index);
        println!("Removed a library\nLibraries: {:?}", self.directories);

        let _ = library_tx.send(LibraryRequest::RegisterUndoDirectory(removed_dir.clone()));
        let _ = library_tx.send(LibraryRequest::Rebuild);

        let _ = ui_tx().send(UpdateUI::Notification(
            format!("Removed a library directory: {removed_dir}"),
            Some(Box::new(move || {
                let _ = library_tx.send(LibraryRequest::UndoRemovedDirectory(removed_dir.clone()));
            })),
        ));
        let _ = ui_tx().send(UpdateUI::SetLibraryDirs(self.directories.clone().into()));

        self.update_trim_uri();
    }

    /// Requests a library rebuild and updates the directory list in the UI
    fn update_library(&self) {
        let _ = ui_tx().send(UpdateUI::SetLibraryDirs(self.directories.clone().into()));

        let library_tx = library_tx();
        let _ = library_tx.send(LibraryRequest::CancelRebuild);
        let _ = library_tx.send(LibraryRequest::Rebuild);
    }

    #[inline]
    fn update_uris(&mut self) {
        self.directory_uris = (self.directories.iter())
            .map(|dir| gio::File::for_path(dir).uri())
            .collect();
    }

    /// Updates the `uri_opt` property, used to optimize song index lookups
    ///
    /// For example, for `["file:///home/Music", "file:///home/Other"]`,
    /// the common part is `"file:///home/"`, so `uri_opt` becomes 13
    ///
    /// Note: If spaces or special characters are common between directories,
    /// the assigned value may be shorter than necessary
    #[inline]
    pub fn update_trim_uri(&mut self) {
        match self.directories.len() {
            1 => return self.uri_opt = self.directory_uris[0].len(),
            0 => return self.uri_opt = 0,
            _ => self.uri_opt = 0,
        }

        let mut dirs: Vec<Chars> = self.directory_uris.iter().map(|dir| dir.chars()).collect();
        'counter: loop {
            let mut chars = dirs.iter_mut().map(|c| c.next());
            let Some(mut adj) = chars.next().unwrap_or(None) else {
                break 'counter;
            };
            for cur in chars {
                let Some(cur) = cur else {
                    break 'counter;
                };
                if cur != adj {
                    break 'counter;
                }
                adj = cur;
            }
            self.uri_opt += 1;
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
        fs::create_dir_all(config_dir()).expect("Could not create the config directory");
    }
}
