use rand::random_range;
use std::fs;
use std::sync::mpsc;
use tokio::sync::mpsc as tokio_mpsc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::player::{PlayerRequest, queue_item::QueueItem};
use crate::ui::UpdateUI;
use crate::{CONFIG_DIR, ReorderVecExt};

pub struct SongQueue {
    repeat: bool,
    shuffle: bool,

    pub pending_track: bool,

    index: usize,
    songs: Vec<QueueItem>,
    shuffled: Vec<usize>,

    player_tx: mpsc::Sender<PlayerRequest>,
    ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
}

impl SongQueue {
    #[must_use]
    pub const fn new(
        player_tx: mpsc::Sender<PlayerRequest>,
        ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
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

    /// Returns the current queue position index
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Locates a song within the `shuffled` vec and returns its index
    #[must_use]
    pub fn shuffled_index(&self, index: usize) -> Option<usize> {
        (0..self.shuffled.len()).find(|i| self.shuffled[*i] == index)
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
        (0..self.len()).map(|i| self.nth(i).clone()).collect()
    }

    /// Replaces the current queue with the provided one
    /// and optionally enables shuffle mode and sets the
    /// arrangement as provided by `shuffled`
    pub fn init_new(&mut self, queue: Vec<QueueItem>, shuffled: Option<Vec<usize>>) {
        self.songs = queue;
        match shuffled {
            Some(shuffled) => {
                self.shuffle = true;
                self.shuffled = shuffled;
            }
            None => self.shuffle = false,
        }
        self.ui_update_shuffle();
        self.ui_update_queue();
    }

    /// Replaces the current queue with the provided one
    pub fn load_new(&mut self, queue: Vec<QueueItem>) {
        self.songs = queue;
        match self.shuffle {
            true => self.new_shuffled_queue(),
            false => self.ui_update_queue(),
        }
    }

    /// Restarts the queue from the beginning
    /// Playback state has to be manually updated
    ///
    /// # Panics
    /// The function panics if the player channel receiver is closed
    pub fn restart_queue(&mut self) {
        self.player_tx.send(PlayerRequest::SkipTo(0)).expect(EXP_RX);
    }

    /// Creates a vec of random indexes for the shuffle mode
    fn new_shuffled_queue(&mut self) {
        self.shuffled = (0..self.len()).collect();
        for i in 0..self.shuffled.len() {
            let rand_index = random_range(0..self.shuffled.len());
            self.shuffled.swap(i, rand_index);
        }
        self.ui_update_queue();
    }

    /// Randomizes the shuffled queue without
    /// changing the currently playing track
    fn update_shuffled_queue(&mut self) {
        if self.is_empty() {
            return;
        }
        if !self.shuffle {
            self.ui_update_queue();
            return;
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
    }

    /// Removes all upcomming songs from the queue
    pub fn clear_queue(&mut self) {
        let current_song = self.remove(self.current_index());
        self.songs = vec![current_song];
        self.shuffled = vec![0];
        self.ui_update_queue();
    }

    /// Moves a song in the queue from `from` to `to`
    pub fn reorder(&mut self, from: usize, to: usize) {
        if self.shuffle {
            self.shuffled.reorder(from, to);
        } else {
            self.songs.reorder(from, to);
        }

        if self.index == from {
            self.index = to;
        } else if from < to && (from..=to).contains(&self.index) {
            self.index -= 1;
        } else if from > to && (to..from).contains(&self.index) {
            self.index += 1;
        }

        self.ui_update_queue();
    }

    /// Inserts an item into the queue at the specified index
    pub fn insert(&mut self, mut index: usize, item: QueueItem) {
        if index < self.len() && self.nth(index).is_stopper() {
            if item.is_stopper() {
                return; // Disallow inserting duplicate stoppers
            }
            index += 1; // Keep stopper after the same song as before
        }

        if self.shuffle {
            self.songs.push(item);
            self.shuffled.insert(index, self.len() - 1);
        } else {
            self.songs.insert(self.ordered_index(index), item);
        }

        if self.index >= index {
            self.index += 1;
        }

        self.ui_update_queue();
    }

    /// Adds an item to the end of the queue
    pub fn add(&mut self, item: QueueItem) {
        self.songs.push(item);
        self.shuffled.push(self.len() - 1);
        self.ui_update_queue();
    }

    /// Appends multiple items to the end of the current queue
    /// UI queue must be manually updated
    pub fn append(&mut self, items: &[QueueItem]) {
        self.songs.extend_from_slice(items);
        // self.songs = [self.songs.as_slice(), items].concat();
        self.ui_update_queue();
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
            // A new shuffled queue is created when shuffle mode is enabled,
            // so only the regular queue must be updated here
            self.songs.remove(index)
        };
        if index < self.index {
            self.index -= 1;
            // self.ui_update_queue_index();
        }
        self.ui_update_queue();
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

    /// Removes all queued songs after the provided index
    /// Index depends on shuffle mode (use `ordered_queue()` index)
    pub fn remove_all_after_index(&mut self, index: usize) {
        while self.len() > index + 1 {
            self.remove(self.len() - 1);
        }
        self.ui_update_queue();
    }

    /// Returns `true` if the current song is first in the queue
    #[must_use]
    pub const fn is_first(&self) -> bool {
        self.index == 0
    }

    /// Returns `true` if the current song is last in the queue
    #[must_use]
    pub const fn is_last(&self) -> bool {
        self.index == self.len() - 1
    }

    /// Returns `true` if there are more tracks in the queue,
    /// or `false` if there is nothing to play afterwrads
    /// Always returns `true` if `repeat` is enabeld
    #[must_use]
    pub const fn has_next(&self) -> bool {
        self.repeat || !self.is_last()
    }

    /// Returns the total number of songs in the queue
    ///
    /// Note: Do not use to index into `shuffled` when shuffle is disabled
    #[must_use]
    pub const fn len(&self) -> usize {
        self.songs.len()
    }

    /// Returns `true` if the queue contains no songs
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    /// Enables or disables shuffle mode for the queue
    pub fn set_shuffle(&mut self, shuffle: bool) {
        // TODO: Keep stoppers in the same place in the queue when toggling shuffle
        if self.shuffle == shuffle {
            return;
        }
        if !shuffle && !self.is_empty() {
            self.index = self.current_index();
        }
        self.shuffle = shuffle;
        self.ui_update_shuffle();
        self.update_shuffled_queue();
    }

    /// Returns the current shuffle mode for the queue
    #[must_use]
    pub const fn get_shuffle(&self) -> bool {
        self.shuffle
    }

    /// Enables or disables repeat mode for the queue
    pub fn set_repeat(&mut self, repeat: bool) {
        if self.repeat == repeat {
            return;
        }
        self.repeat = repeat;
        self.ui_update_repeat();
    }

    /// Returns the current repeat mode for the queue
    #[must_use]
    pub const fn get_repeat(&self) -> bool {
        self.repeat
    }

    /// Updates the UI with the current queue shuffle mode setting
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    fn ui_update_shuffle(&self) {
        println!("ui_update_shuffle({})", self.shuffle);
        self.ui_tx
            .send(UpdateUI::Shuffle(self.shuffle))
            .expect(EXP_RX);
    }

    /// Updates the UI with the current queue repeat mode setting
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    fn ui_update_repeat(&self) {
        println!("ui_update_repeat({})", self.repeat);
        self.ui_tx
            .send(UpdateUI::Repeat(self.repeat))
            .expect(EXP_RX);
    }

    /// Updates the UI with the current queue
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    fn ui_update_queue(&self) {
        println!("ui_update_queue()");
        self.ui_update_queue_index();
        self.ui_tx
            .send(UpdateUI::NewQueue(self.ordered_queue()))
            .expect(EXP_RX);
    }

    /// Updates the UI with the current queue index
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    pub fn ui_update_queue_index(&self) {
        println!("ui_update_queue_index({})", self.index);
        self.ui_tx
            .send(UpdateUI::QueueIndex(self.index))
            .expect(EXP_RX);
    }

    pub fn send_both_queues(&self, tx: mpsc::Sender<(usize, Vec<QueueItem>, Vec<usize>, bool)>) {
        // NOTE: Could also use `mem::take` instead of `clone`
        // if this only ever gets called during shutdown
        tx.send((
            self.index,
            self.songs.clone(),
            self.shuffled.clone(),
            self.shuffle,
        ))
        .expect(EXP_RX);
    }

    /// Saves the provided queue to a file on disk, or removes
    /// the file if `remember` is `false`
    ///
    /// # Panics
    /// The function panics if `CONFIG_DIR` is unititialized
    pub fn save_queue(remember: bool, playing_index: usize, song_queue: &[QueueItem]) {
        let queue_file = CONFIG_DIR.get().expect(EXP_INIT).to_owned() + "queue";
        if !remember {
            let _ = fs::remove_file(&queue_file);
            return;
        }
        let contents = playing_index.to_string()
            + "\n"
            + song_queue
                .iter()
                .map(|item| match item {
                    QueueItem::Song(song) => song.info().file_path() + "\n",
                    QueueItem::Stopper => String::from("Stopper\n"),
                })
                .collect::<String>()
                .trim();
        match fs::write(&queue_file, contents) {
            Ok(()) => println!("Song queue state successfully written to disk"),
            Err(e) => eprintln!("Problems writing queue state: {e}"),
        }
    }
    pub fn save_shuffled_queue(shuffle_mode: bool, shuffled_queue: &[usize]) {
        let shuffled_file = CONFIG_DIR.get().expect(EXP_INIT).to_owned() + "queue_shuffled";
        if !shuffle_mode {
            let _ = fs::remove_file(&shuffled_file);
            return;
        }
        let contents = shuffled_queue
            .iter()
            .map(|i| i.to_string() + "\n")
            .collect::<String>();
        match fs::write(&shuffled_file, contents) {
            Ok(()) => println!("Shuffled song queue successfully written to disk"),
            Err(e) => eprintln!("Problems writing queue state: {e}"),
        }
    }
}
