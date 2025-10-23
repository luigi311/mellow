use rand::random_range;
use std::sync::mpsc;

use crate::library::Song;
use crate::player::PlayerRequest;

pub struct SongQueue {
    pub repeat: bool,
    pub shuffle: bool,

    pub pending_track: bool,
    pub end_of_queue: bool,

    index: usize,
    songs: Vec<QueueItem>,
    shuffled: Vec<usize>,

    player_tx: mpsc::SyncSender<PlayerRequest>,
}

pub enum QueueItem {
    Song(Song),
    Stopper,
}

impl QueueItem {
    pub fn as_ref_song(&self) -> &Song {
        match self {
            QueueItem::Song(song) => song,
            _ => panic!("Item is not a `Song`"),
        }
    }
    pub fn as_mut_song(&mut self) -> &mut Song {
        match self {
            QueueItem::Song(song) => song,
            _ => panic!("Item is not a `Song`"),
        }
    }
}

impl SongQueue {
    #[must_use]
    pub fn new(player_tx: mpsc::SyncSender<PlayerRequest>) -> Self {
        Self {
            repeat: false,
            shuffle: false,

            pending_track: true,
            end_of_queue: false,

            index: 0,
            songs: vec![],
            shuffled: vec![],

            player_tx,
        }
    }

    /// Moves to the next song in the queue
    pub fn next(&mut self) {
        self.index += 1;
        if self.index == self.len() {
            self.index = 0;
            self.end_of_queue = !self.repeat;
            self.pending_track &= !self.end_of_queue;
        }
    }

    /// Moves to the previous song in the queue
    pub fn previous(&mut self) {
        if self.index == 0 {
            if self.repeat {
                self.index = self.len() - 1;
            }
            return;
        }
        self.index -= 1;
    }

    /// Moves to the song in the queue at specified index
    pub fn jump_to(&mut self, index: usize) {
        self.index = index;
    }

    /// Get the current song index based on the shuffle mode option
    #[must_use]
    pub fn get_current_index(&self) -> usize {
        self.ordered_index(self.index)
    }

    /// Returns a mutable reference to the current song
    #[must_use]
    pub fn get_current(&mut self) -> &mut QueueItem {
        let index = self.get_current_index();
        &mut self.songs[index]
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
    pub fn ordered_queue(&self) -> Vec<&QueueItem> {
        let mut songs = vec![];
        for i in 0..self.len() {
            songs.push(&self.songs[self.ordered_index(i)]);
        }
        songs
    }

    /// Replaces the current queue with the provided one
    /// Index depends on shuffle mode (use `ordered_queue()` index)
    /// Playback state has to be manually updated
    pub fn replace(&mut self, queue: Vec<QueueItem>) -> Result<(), mpsc::SendError<PlayerRequest>> {
        self.player_tx.send(PlayerRequest::SetInstantURI(true))?;
        self.pending_track = true;
        self.songs = queue;
        self.new_shuffled_queue();
        Ok(())
    }

    /// Restarts the queue from the beginning
    /// Playback state has to be manually updated
    pub fn restart_queue(&mut self) -> Result<(), mpsc::SendError<PlayerRequest>> {
        self.player_tx.send(PlayerRequest::SetInstantURI(true))?;
        self.pending_track = true;
        self.index = 0;
        Ok(())
    }

    /// Enables or disables the shuffle mode
    pub fn set_shuffle(&mut self, shuffle: bool) {
        // TODO: Keep stoppers in the same place in the queue when toggling shuffle
        if self.shuffle == shuffle {
            return;
        }
        if self.shuffle {
            self.index = self.get_current_index();
        }
        self.shuffle = shuffle;
        self.update_shuffled_queue();
    }

    /// Creates a vec of random indexes for the shuffle mode
    fn new_shuffled_queue(&mut self) {
        self.shuffled = (0..self.len()).collect();
        for i in 0..self.shuffled.len() {
            let rand_index = random_range(0..self.shuffled.len());
            self.shuffled.swap(i, rand_index);
        }
    }

    /// Randomizes indexes for the shuffle mode
    /// without changing the currently playing track
    fn update_shuffled_queue(&mut self) {
        if self.is_empty() {
            eprintln!("Cannot shuffle an empty queue");
            return;
        }
        self.shuffled = (0..self.len()).collect();
        let start = match self.get_current_index() {
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
    }

    /// Removes all upcomming songs from the queue
    pub fn clear_queue(&mut self) {
        let current_song = self.remove(self.get_current_index());
        self.songs = vec![current_song];
        self.update_shuffled_queue();
    }

    /// Removes all queued songs after the provided index
    /// Index depends on shuffle mode (use `ordered_queue()` index)
    pub fn clear_queue_after_index(&mut self, index: usize) {
        while self.len() > index + 1 {
            self.remove(index + 1);
        }
    }

    /// Moves a song in the queue from `index` to `target`
    /// Index depends on shuffle mode (use `ordered_queue()` index)
    pub fn reorder(&mut self, index: usize, target: usize) {
        if self.shuffle {
            let item = self.shuffled.remove(index);
            self.shuffled.insert(target, item);
        } else {
            let item = self.songs.remove(index);
            self.songs.insert(target, item);
        }
    }

    /// Inserts an item into the queue at the specified index
    pub fn insert(&mut self, index: usize, item: QueueItem) {
        let ordered_index = self.ordered_index(index);
        self.songs.insert(ordered_index, item);
        if self.shuffle {
            self.shuffled.insert(index, ordered_index);
        }
    }

    /// Removes a song from the queue at the specified index
    /// Index depends on shuffle mode (use `ordered_queue()` index)
    /// Returns the song which was previously located at that index
    pub fn remove(&mut self, index: usize) -> QueueItem {
        if self.shuffle {
            self.songs.remove(self.shuffled.remove(index))
        } else {
            self.shuffled.remove(self.shuffled_index(index).unwrap());
            self.songs.remove(index)
        }
    }

    /// Removes the current song from the queue
    pub fn remove_current(&mut self) -> QueueItem {
        self.remove(self.index)
    }

    /// Returns the total number of songs in the queue
    #[must_use]
    pub fn len(&self) -> usize {
        self.songs.len()
    }

    /// Returns `true` if the current song is first in the queue
    #[must_use]
    pub fn is_first(&self) -> bool {
        self.index == 0
    }

    /// Returns `true` if the current song is last in the queue
    #[must_use]
    pub fn is_last(&self) -> bool {
        self.index == self.songs.len() - 1
    }

    /// Returns `true` if the queue contains no songs
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }
}
