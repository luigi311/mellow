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
    songs: Vec<Song>,
    shuffled: Vec<usize>,

    player_tx: mpsc::SyncSender<PlayerRequest>,
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

    /// Get the current song index based on the shuffle mode option
    #[must_use]
    pub fn get_current_index(&self) -> usize {
        match self.shuffle {
            true => self.shuffled[self.index],
            false => self.index,
        }
    }

    /// Returns a mutable reference to the current song
    #[must_use]
    pub fn get_current(&mut self) -> &mut Song {
        match self.shuffle {
            true => &mut self.songs[self.shuffled[self.index]],
            false => &mut self.songs[self.index],
        }
    }

    pub fn next(&mut self) {
        self.index += 1;
        if self.index == self.len() {
            self.index = 0;
            self.end_of_queue = !self.repeat;
            self.pending_track &= !self.end_of_queue;
        }
    }

    pub fn previous(&mut self) {
        if self.index == 0 {
            if self.repeat {
                self.index = self.len() - 1;
            }
            return;
        }
        self.index -= 1;
    }

    pub fn jump_to(&mut self, index: usize) {
        self.index = index;
    }

    /// Replaces the current queue with the provided one
    /// Playback state has to be manually updated
    pub fn replace(&mut self, queue: Vec<Song>) -> Result<(), mpsc::SendError<PlayerRequest>> {
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

    pub fn set_shuffle(&mut self, shuffle: bool) {
        if self.shuffle == shuffle {
            return;
        }
        if self.shuffle {
            self.index = self.get_current_index();
        }
        self.shuffle = shuffle;
        self.update_shuffled_queue();
    }

    fn new_shuffled_queue(&mut self) {
        self.shuffled = (0..self.len()).collect();
        for i in 0..self.shuffled.len() {
            let rand_index = random_range(0..self.shuffled.len());
            self.shuffled.swap(i, rand_index);
        }
    }

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
    pub fn clear_queue_after_index(&mut self, index: usize) {
        while self.len() > index + 1 {
            self.remove(index + 1);
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.songs.len()
    }

    #[must_use]
    pub fn is_first(&self) -> bool {
        self.index == 0
    }

    #[must_use]
    pub fn is_last(&self) -> bool {
        self.index == self.songs.len() - 1
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    #[must_use]
    pub fn index_to_shuffled(&self, index: usize) -> Option<usize> {
        for i in 0..self.len() {
            if self.shuffled[i] == index {
                return Some(index);
            }
        }
        None
    }

    #[must_use]
    pub fn shuffled_to_index(&self, index: usize) -> usize {
        self.shuffled[index]
    }

    pub fn remove(&mut self, index: usize) -> Song {
        self.shuffled.remove(self.index_to_shuffled(index).unwrap());
        self.songs.remove(index)
    }

    pub fn remove_shuffled(&mut self, index: usize) -> Song {
        self.songs.remove(self.shuffled.remove(index))
    }
}
