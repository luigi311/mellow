use std::sync::Arc;

use crate::library::{Song, song::SharedSong};

#[derive(Clone)]
pub enum QueueItem {
    Song(SharedSong),
    Stopper,
}

impl QueueItem {
    /// Assumes the `QueueItem` is a `Song`, and returns a
    /// `MutexGuard` for accessing the inner value
    ///
    /// # Panics
    /// The function panics if the `QueueItem` is not a `Song`
    ///
    /// Note: Since each `Stopper` is removed when encountered,
    /// this method is safe when chained with `Song::current()`
    #[must_use]
    pub fn as_song(&self) -> &Song {
        match self {
            Self::Song(song) => song,
            Self::Stopper => panic!("called `QueueItem::as_song()` on a `Stopper` value"),
        }
    }
    /// Returns `true` if the `QueueItem` is a `Song`
    #[must_use]
    pub const fn is_song(&self) -> bool {
        matches!(self, Self::Song(_))
    }
    /// Returns `true` if the `QueueItem` is a `Stopper`
    #[must_use]
    pub const fn is_stopper(&self) -> bool {
        matches!(self, Self::Stopper)
    }

    /// Runs a closure on the `QueueItem` if it is a `Song`,
    /// and returns the output of the closure inside an `Option`.
    /// If the `QueueItem` is not a `Song`, `None` is returned.
    pub fn map<F, T>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&Song) -> T,
    {
        match self {
            QueueItem::Song(song) => Some(f(song)),
            _ => None,
        }
    }

    /// Creates a `QueueItem::Song` using the specified `song`
    #[inline]
    #[must_use]
    pub fn from_song(song: &SharedSong) -> QueueItem {
        QueueItem::Song(Arc::clone(song))
    }
}
