use rand::random_range;
use std::fs;
use std::sync::mpsc;
use tokio::sync::mpsc as tokio_mpsc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::player::{PlayerRequest, queue_item::QueueItem};
use crate::ui::UpdateUI;
use crate::{CONFIG_DIR, reorder_vec};

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
        for i in 0..self.shuffled.len() {
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
        let mut songs = Vec::with_capacity(self.len());
        for i in 0..self.len() {
            songs.push(self.nth(i).clone());
        }
        songs.into()
    }

    /// Replaces the current queue with the provided one
    pub fn load_new(&mut self, queue: Vec<QueueItem>) {
        self.songs = queue;
        match self.shuffle {
            true => self.new_shuffled_queue(),
            false => self.ui_update_queue(),
        }
        if self.is_empty() {
            self.ui_open_playing();
        }
    }

    /// Restarts the queue from the beginning
    /// Playback state has to be manually updated
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

    /// Randomizes indexes for the shuffle mode
    /// without changing the currently playing track
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
        self.ui_update_queue();
    }

    /// Removes all upcomming songs from the queue
    pub fn clear_queue(&mut self) {
        let current_song = self.remove(self.current_index());
        self.songs = vec![current_song];
        self.update_shuffled_queue();
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
    pub fn insert(&mut self, index: usize, item: QueueItem) {
        if item.is_stopper() && index < self.len() && self.nth(index).is_stopper() {
            return;
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

    fn ui_update_shuffle(&self) {
        println!("ui_update_shuffle({})", self.shuffle);
        self.ui_tx
            .send(UpdateUI::Shuffle(self.shuffle))
            .expect(EXP_RX);
    }

    fn ui_update_repeat(&self) {
        println!("ui_update_repeat({})", self.repeat);
        self.ui_tx
            .send(UpdateUI::Repeat(self.repeat))
            .expect(EXP_RX);
    }

    fn ui_update_queue(&self) {
        println!("ui_update_queue()");
        self.ui_update_queue_index();
        self.ui_tx
            .send(UpdateUI::NewQueue(self.ordered_queue()))
            .expect(EXP_RX);
    }

    pub fn ui_update_queue_index(&self) {
        println!("ui_update_queue_index({})", self.index);
        self.ui_tx
            .send(UpdateUI::QueueIndex(self.index))
            .expect(EXP_RX);
    }

    /// Requests the UI to open the music library
    fn ui_open_playing(&self) {
        self.ui_tx.send(UpdateUI::FocusPlaying).expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(true)).expect(EXP_RX);
    }

    /// Saves the provided queue to a file on disk, or removes
    /// the file if `remember` is `false`
    pub fn save_queue(remember: bool, song_queue: &[QueueItem], playing_index: usize) {
        let queue_file = CONFIG_DIR.get().expect(EXP_INIT).to_owned() + "queue";
        let shuffled_file = CONFIG_DIR.get().expect(EXP_INIT).to_owned() + "queue_shuffled";
        if !remember {
            let _ = fs::remove_file(&queue_file);
            let _ = fs::remove_file(&shuffled_file);
            return;
        }
        let contents = playing_index.to_string()
            + "\n"
            + song_queue
                .iter()
                .map(|item| match item {
                    QueueItem::Song(song) => song.lock().unwrap().info().file_path() + "\n",
                    QueueItem::Stopper => String::from("Stopper\n"),
                })
                .collect::<String>()
                .trim();
        // TODO: Also save the shuffled queue and shuffle setting (new file)
        // the file contents could look something like this:
        // /------------------\
        // | True             | <- Whether shuffle mode is on
        // | 50,32,67,4,89,22,| <- Shuffled indexes for the player queue
        // \------------------/
        match fs::write(&queue_file, contents) {
            Ok(()) => println!("Song queue state successfully written to disk"),
            Err(e) => eprintln!("Problems writing queue state: {e}"),
        }
    }
}
