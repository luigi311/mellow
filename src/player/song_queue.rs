use core::error::Error;
use rand::random_range;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::library::Song;
use crate::player::PlayerRequest;
use crate::ui::UpdateUI;

pub struct SongQueue {
    repeat: bool,
    shuffle: bool,

    pub pending_track: bool,
    pub end_of_queue: bool,
    pub lock_current: bool,

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
    pub fn as_song(&self) -> MutexGuard<'_, Song> {
        match self {
            QueueItem::Song(song) => song.lock().unwrap(),
            _ => panic!("Item is not a `Song`"),
        }
    }
}

impl SongQueue {
    #[must_use]
    pub fn new(
        player_tx: mpsc::SyncSender<PlayerRequest>,
        ui_tx: tokio_mpsc::Sender<UpdateUI>,
        tokio_rt: Arc<tokio::runtime::Runtime>,
    ) -> Self {
        Self {
            repeat: false,
            shuffle: false,

            pending_track: true,
            end_of_queue: false,
            lock_current: false,

            index: 0,
            songs: vec![],
            shuffled: vec![],

            player_tx,
            ui_tx,
            tokio_rt,
        }
    }

    /// Moves to the next song in the queue
    pub fn next(&mut self) {
        self.index += 1;
        if self.index == self.len() {
            self.index = 0;
            self.end_of_queue = !self.repeat;
            // self.pending_track &= !self.end_of_queue;
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
    pub fn current_index(&self) -> usize {
        self.ordered_index(self.index)
    }

    /// Returns a mutable reference to the current song
    #[must_use]
    pub fn current(&mut self) -> &mut QueueItem {
        let index = self.current_index();
        &mut self.songs[index]
    }

    #[must_use]
    pub fn index(&self) -> usize {
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
            songs.push(self.songs[self.ordered_index(i)].clone());
        }
        songs.into()
    }

    /// Replaces the current queue with the provided one
    /// Shuffle/repeat can be set by providing the `Some(_)` value,
    /// otherwise previous values are preserved
    /// Playback state has to be manually updated
    pub fn load_new(
        &mut self,
        queue: Vec<QueueItem>,
        shuffle: Option<bool>,
        repeat: Option<bool>,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(shuffle) = shuffle {
            self.set_shuffle(shuffle)?;
        }
        if let Some(repeat) = repeat {
            self.set_repeat(repeat)?;
        }

        self.player_tx.send(PlayerRequest::SetInstantURI(true))?;
        self.pending_track = true;
        self.songs = queue;
        self.index = 0;
        self.new_shuffled_queue()?;

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

    /// Enables or disables shuffle mode for the queue
    pub fn set_shuffle(&mut self, shuffle: bool) -> Result<(), SendError<UpdateUI>> {
        // TODO: Keep stoppers in the same place in the queue when toggling shuffle
        if self.shuffle == shuffle {
            return Ok(());
        }
        if self.shuffle && !self.is_empty() {
            self.index = self.current_index();
        }
        self.shuffle = shuffle;
        self.ui_update_shuffle()?;
        self.update_shuffled_queue()?;
        Ok(())
    }

    /// Returns the current shuffle mode for the queue
    pub fn get_shuffle(&self) -> bool {
        self.shuffle
    }

    /// Enables or disables repeat mode for the queue
    pub fn set_repeat(&mut self, repeat: bool) -> Result<(), SendError<UpdateUI>> {
        self.repeat = repeat;
        self.ui_update_shuffle()?;
        Ok(())
    }

    /// Returns the current repeat mode for the queue
    pub fn get_repeat(&self) -> bool {
        self.repeat
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
    pub fn clear_queue_after_index(&mut self, index: usize) -> Result<(), SendError<UpdateUI>> {
        while self.len() > index + 1 {
            self.remove(index + 1);
        }
        self.ui_update_queue()?;
        Ok(())
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
        if self.index + 1 == self.len() {
            self.index = 0;
            self.end_of_queue = !self.repeat;
        }
        self.pending_track = true;
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

    fn ui_update_shuffle(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        println!("ui_update_shuffle({})", self.shuffle);
        self.ui_update_queue_index()?;
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::Shuffle(self.shuffle)).await })
    }

    fn ui_update_repeat(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        println!("ui_update_repeat({})", self.repeat);
        self.ui_update_queue_index()?;
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
}
