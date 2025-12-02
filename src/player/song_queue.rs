use core::error::Error;
use rand::random_range;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::library::Song;
use crate::player::PlayerRequest;
use crate::reorder_vec;
use crate::ui::UpdateUI;

pub struct SongQueue {
    repeat: bool,
    shuffle: bool,

    pub pending_track: bool,

    index: usize,
    songs: Vec<QueueItem>,
    shuffled: Vec<usize>,

    player_tx: mpsc::SyncSender<PlayerRequest>,
    ui_tx: tokio_mpsc::Sender<UpdateUI>,
    tokio_rt: Arc<tokio::runtime::Runtime>,
}

#[derive(Clone)]
pub enum QueueItem {
    Song(Arc<Mutex<Song>>),
    Stopper,
}

impl QueueItem {
    /// Assumes the `QueueItem` is a `Song`, and returns a
    /// `MutexGuard` for accessing the inner value
    ///
    /// # Panics
    /// The function panics if the `QueueItem` is not a `Song`
    pub fn as_song(&self) -> MutexGuard<'_, Song> {
        match self {
            Self::Song(song) => song.lock().unwrap(),
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
}

impl SongQueue {
    #[must_use]
    pub const fn new(
        player_tx: mpsc::SyncSender<PlayerRequest>,
        ui_tx: tokio_mpsc::Sender<UpdateUI>,
        tokio_rt: Arc<tokio::runtime::Runtime>,
    ) -> Self {
        Self {
            repeat: false,
            shuffle: false,

            pending_track: true,

            index: 0,
            songs: vec![],
            shuffled: vec![],

            player_tx,
            ui_tx,
            tokio_rt,
        }
    }

    /// Moves to the next song in the queue
    pub const fn move_next(&mut self) {
        match self.is_last() {
            false => self.index += 1,
            true => self.index = 0,
        }
    }

    /// Moves to the previous song in the queue
    pub const fn move_previous(&mut self) {
        match self.is_first() {
            false => self.index -= 1,
            true if self.repeat => self.index = self.len() - 1,
            true => (),
        }
    }

    /// Moves to the song in the queue at specified index
    pub const fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    /// Returns a mutable reference to the current song
    #[must_use]
    pub fn current(&mut self) -> &mut QueueItem {
        let index = self.current_index();
        &mut self.songs[index]
    }

    /// Returns a reference to the next item in the queue
    #[must_use]
    pub fn next(&self) -> Option<&QueueItem> {
        if self.is_last() {
            return None;
        }
        Some(self.nth(self.index + 1))
    }

    /// Returns a reference to the previous item in the queue
    #[must_use]
    pub fn previous(&self) -> Option<&QueueItem> {
        if self.is_first() {
            return None;
        }
        Some(self.nth(self.index - 1))
    }

    /// Returns a reference to the `n`th item in the queue,
    /// respecting the shuffle mode setting
    ///
    /// # Panics
    /// The function panics if `n` is out of bounds
    #[must_use]
    pub fn nth(&self, n: usize) -> &QueueItem {
        &self.songs[self.ordered_index(n)]
    }

    /// Returns the current song index based on the shuffle mode option
    #[must_use]
    pub fn current_index(&self) -> usize {
        self.ordered_index(self.index)
    }

    /// Returns the current index used internally by the queue
    /// When indexing into a shuffled or sequential list (such as
    /// for display by the UI), use `ordered_index()` instead
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Locates a song within the `shuffled` vec and returns its index
    #[must_use]
    pub fn shuffled_index(&self, index: usize) -> Option<usize> {
        for i in 0..self.len() {
            if self.shuffled[i] == index {
                return Some(index);
            }
        }
        None
    }

    /// Turns an index from `shuffled` into one which can be used with `songs`.
    /// If the shuffle mode is off, the input index is returned.
    ///
    /// # Panics
    /// The function panics if `index` is out of bounds
    #[must_use]
    pub fn ordered_index(&self, index: usize) -> usize {
        match self.shuffle {
            true => self.shuffled[index],
            false => index,
        }
    }

    /// Returns references to all songs in the queue,
    /// ordered with respect to shuffle setting
    #[must_use]
    pub fn ordered_queue(&self) -> Box<[QueueItem]> {
        let mut songs = vec![];
        for i in 0..self.len() {
            songs.push(self.nth(i).clone());
        }
        songs.into()
    }

    /// Replaces the current queue with the provided one
    pub fn load_new(&mut self, queue: Vec<QueueItem>) -> Result<(), Box<dyn Error>> {
        self.songs = queue;
        self.new_shuffled_queue()?;
        self.player_tx.send(PlayerRequest::SkipTo(0))?;

        if self.is_empty() {
            self.ui_open_library()?;
        }

        Ok(())
    }

    /// Restarts the queue from the beginning
    /// Playback state has to be manually updated
    pub fn restart_queue(&mut self) -> Result<(), mpsc::SendError<PlayerRequest>> {
        self.player_tx.send(PlayerRequest::SkipTo(0))
    }

    /// Creates a vec of random indexes for the shuffle mode
    fn new_shuffled_queue(&mut self) -> Result<(), SendError<UpdateUI>> {
        self.shuffled = (0..self.len()).collect();
        for i in 0..self.shuffled.len() {
            let rand_index = random_range(0..self.shuffled.len());
            self.shuffled.swap(i, rand_index);
        }
        self.ui_update_queue()?;
        Ok(())
    }

    /// Randomizes indexes for the shuffle mode
    /// without changing the currently playing track
    fn update_shuffled_queue(&mut self) -> Result<(), SendError<UpdateUI>> {
        if self.is_empty() {
            return Ok(());
        }
        if !self.shuffle {
            self.ui_update_queue()?;
            return Ok(());
        }
        self.shuffled = (0..self.len()).collect();
        let start = match self.current_index() {
            index if self.shuffle => {
                self.shuffled.swap(0, index);
                self.index = 0;
                1
            }
            _ => 0,
        };
        for i in start..self.shuffled.len() {
            let rand_index = random_range(start..self.shuffled.len());
            self.shuffled.swap(i, rand_index);
        }
        self.ui_update_queue()?;
        Ok(())
    }

    /// Removes all upcomming songs from the queue
    pub fn clear_queue(&mut self) -> Result<(), SendError<UpdateUI>> {
        let current_song = self.remove(self.current_index());
        self.songs = vec![current_song];
        self.update_shuffled_queue()?;
        Ok(())
    }

    /// Removes all queued songs after the provided index
    /// Index depends on shuffle mode (use `ordered_queue()` index)
    pub fn remove_all_after_index(&mut self, index: usize) -> Result<(), SendError<UpdateUI>> {
        while self.len() > index + 1 {
            self.remove(self.len() - 1);
        }
        self.ui_update_queue()?;
        Ok(())
    }

    /// Moves a song in the queue from `index` to `target`
    /// Index depends on shuffle mode (use `ordered_queue()` index)
    pub fn reorder(&mut self, index: usize, target: usize) {
        if self.shuffle {
            reorder_vec(&mut self.shuffled, index, target);
        } else {
            reorder_vec(&mut self.songs, index, target);
        }
        // TODO: Test if this works
        if self.index == index {
            self.index = target;
        } else if index < target && (index..=target).contains(&self.index) {
            self.index -= 1;
        } else if index > target && (target..index).contains(&self.index) {
            self.index += 1;
        }
    }

    /// Inserts an item into the queue at the specified index
    pub fn insert(&mut self, index: usize, item: QueueItem) -> Result<(), SendError<UpdateUI>> {
        if item.is_stopper() && index < self.len() && self.nth(index).is_stopper() {
            return Ok(());
        }

        let ordered_index = self.ordered_index(index);

        self.songs.insert(ordered_index, item);
        if self.shuffle {
            for shuffled in &mut self.shuffled {
                if *shuffled >= ordered_index {
                    *shuffled += 1;
                }
            }
            self.shuffled.insert(index, ordered_index);
        }

        if self.index >= index {
            self.index += 1;
        }

        self.ui_update_queue()
    }

    /// Adds an item to the end of the queue
    pub fn add(&mut self, item: QueueItem) -> Result<(), SendError<UpdateUI>> {
        self.songs.push(item);
        self.shuffled.push(self.len() - 1);
        self.ui_update_queue()
    }

    /// Appends multiple items to the end of the current queue
    /// UI queue must be manually updated
    pub fn append(&mut self, items: &[QueueItem]) -> Result<(), SendError<UpdateUI>> {
        self.songs = [self.songs.as_slice(), items].concat();
        self.ui_update_queue()
    }

    /// Removes a song from the queue at the specified index
    /// Index depends on shuffle mode (use `ordered_queue()` index)
    /// Returns the removed `QueueItem`
    pub fn remove(&mut self, index: usize) -> QueueItem {
        let previous = if self.shuffle {
            let target = self.shuffled[index];
            for i in 0..index {
                if self.shuffled[i] > target {
                    self.shuffled[i] -= 1;
                }
            }
            for i in index + 1..self.shuffled.len() {
                self.shuffled[i - 1] = match self.shuffled[i] {
                    n if n > target => n - 1,
                    n => n,
                };
            }
            self.shuffled.remove(self.shuffled.len() - 1);
            self.songs.remove(target)
        } else {
            // No need to update the shuffled queue, because
            // a new one is created when it is enabled
            // self.shuffled.remove(self.shuffled_index(index).unwrap());
            self.songs.remove(index)
        };
        if index < self.index {
            self.index -= 1;
            // self.ui_update_queue_index().unwrap();
        }
        self.ui_update_queue().unwrap();
        previous
    }

    /// Removes the current song from the queue
    pub fn remove_current(&mut self) -> QueueItem {
        if self.is_last() {
            self.index = 0;
        }
        self.pending_track = true;
        self.remove(self.index)
    }

    /// Returns the total number of songs in the queue
    #[must_use]
    pub const fn len(&self) -> usize {
        self.songs.len()
    }

    /// Returns `true` if the current song is first in the queue
    #[must_use]
    pub const fn is_first(&self) -> bool {
        self.index == 0
    }

    /// Returns `true` if the current song is last in the queue
    #[must_use]
    pub const fn is_last(&self) -> bool {
        self.index == self.songs.len() - 1
    }

    /// Returns `true` if there are more tracks in the queue,
    /// or `false` if there is nothing to play afterwrads
    /// Always returns `true` if `repeat` is enabeld
    #[must_use]
    pub const fn has_next(&self) -> bool {
        self.repeat || !self.is_last()
    }

    /// Returns `true` if the queue contains no songs
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    /// Enables or disables shuffle mode for the queue
    pub fn set_shuffle(&mut self, shuffle: bool) -> Result<(), SendError<UpdateUI>> {
        // TODO: Keep stoppers in the same place in the queue when toggling shuffle
        if self.shuffle == shuffle {
            return Ok(());
        }
        if !shuffle && !self.is_empty() {
            self.index = self.current_index();
        }
        self.shuffle = shuffle;
        self.ui_update_shuffle()?;
        self.update_shuffled_queue()?;
        Ok(())
    }

    /// Returns the current shuffle mode for the queue
    #[must_use]
    pub const fn get_shuffle(&self) -> bool {
        self.shuffle
    }

    /// Enables or disables repeat mode for the queue
    pub fn set_repeat(&mut self, repeat: bool) -> Result<(), SendError<UpdateUI>> {
        if self.repeat == repeat {
            return Ok(());
        }
        self.repeat = repeat;
        self.ui_update_repeat()?;
        Ok(())
    }

    /// Returns the current repeat mode for the queue
    #[must_use]
    pub const fn get_repeat(&self) -> bool {
        self.repeat
    }

    fn ui_update_shuffle(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        println!("ui_update_shuffle({})", self.shuffle);
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::Shuffle(self.shuffle)).await })
    }

    fn ui_update_repeat(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        println!("ui_update_repeat({})", self.repeat);
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::Repeat(self.repeat)).await })
    }

    fn ui_update_queue(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        println!("ui_update_queue()");
        self.ui_update_queue_index()?;
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::SongQueue(self.ordered_queue())).await })
    }

    pub fn ui_update_queue_index(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        println!("ui_update_queue_index({})", self.index);
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::QueueIndex(self.index)).await })
    }

    /// Requests the UI to open the music library
    fn ui_open_library(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::OpenLibrary).await })
    }
}
