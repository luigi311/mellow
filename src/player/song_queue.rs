use core::error::Error;
use gst::ClockTime;
use rand::random_range;
use std::fs;
use std::sync::mpsc;
use tokio::sync::mpsc as tokio_mpsc;

use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::{LIBRARY_TX, Library, LibraryRequest};
use crate::player::{PlayerRequest, QueueItem};
use crate::ui::{StartupQueueChoice, UpdateUI};
use crate::{CONFIG_DIR, util::ReorderVecExt};

pub struct SongQueue {
    repeat: bool,
    shuffle: bool,

    pub pending_track: bool,

    index: usize,
    last_ui_index: usize,
    songs: Vec<QueueItem>,
    shuffled: Vec<usize>,

    player_tx: mpsc::Sender<PlayerRequest>,
    ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
}

impl SongQueue {
    #[inline]
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
            last_ui_index: 0,
            songs: vec![],
            shuffled: vec![],

            player_tx,
            ui_tx,
        }
    }

    /// Moves to the next song in the queue
    #[inline]
    pub const fn move_next(&mut self) {
        match self.is_last() {
            false => self.index += 1,
            true => self.index = 0,
        }
    }

    /// Moves to the previous song in the queue
    #[inline]
    pub const fn move_previous(&mut self) {
        match self.is_first() {
            false => self.index -= 1,
            true if self.repeat => self.index = self.len() - 1,
            true => (),
        }
    }

    /// Moves to the song in the queue at specified index
    #[inline]
    pub const fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    /// Returns a mutable reference to the current song
    #[inline]
    #[must_use]
    pub fn current(&mut self) -> &mut QueueItem {
        let index = self.current_index();
        &mut self.songs[index]
    }

    /// Returns a reference to the next item in the queue
    #[inline]
    #[must_use]
    pub fn next(&self) -> Option<&QueueItem> {
        if self.is_last() {
            return None;
        }
        Some(self.nth(self.index + 1))
    }

    /// Returns a reference to the previous item in the queue
    #[inline]
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
    #[inline]
    #[must_use]
    pub fn nth(&self, n: usize) -> &QueueItem {
        &self.songs[self.ordered_index(n)]
    }

    /// Returns the current song index based on the shuffle mode option
    #[inline]
    #[must_use]
    pub fn current_index(&self) -> usize {
        self.ordered_index(self.index)
    }

    /// Returns the current queue position index
    #[inline]
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Locates a song within the `shuffled` vec and returns its index
    #[inline]
    #[must_use]
    pub fn shuffled_index(&self, index: usize) -> Option<usize> {
        // NOTE: This function is unused. Should it be removed?
        (0..self.shuffled.len()).find(|i| self.shuffled[*i] == index)
    }

    /// Turns an index from `shuffled` into one which can be used with `songs`.
    /// If the shuffle mode is off, the input index is returned.
    ///
    /// # Panics
    /// The function panics if `index` is out of bounds
    #[inline]
    #[must_use]
    pub fn ordered_index(&self, index: usize) -> usize {
        match self.shuffle {
            true => self.shuffled[index],
            false => index,
        }
    }

    /// Returns references to all songs in the queue,
    /// ordered with respect to shuffle setting
    #[inline]
    #[must_use]
    pub fn ordered_queue(&self) -> Box<[QueueItem]> {
        (0..self.len()).map(|i| self.nth(i).clone()).collect()
    }

    /// Replaces the current queue with the provided one
    /// and optionally enables shuffle mode and sets the
    /// shuffled queue to `shuffled`. If `shuffled` is
    /// `Some` but empty, a new one is created.
    ///
    /// Note: `ui_update_queue` must be called manually
    pub fn load_new(&mut self, queue: Vec<QueueItem>, shuffled: Option<Vec<usize>>) {
        self.songs = queue;
        self.repeat = false;
        if let Some(shuffled) = shuffled {
            self.shuffle = true;
            match shuffled.is_empty() {
                false => self.shuffled = shuffled,
                true => self.new_shuffled_queue(),
            }
        } else {
            self.shuffle = false;
        }
        self.ui_update_shuffle();
        self.ui_update_repeat();
        self.ui_close_queue_subpage();
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
    }

    /// Randomizes the shuffled queue without
    /// changing the currently playing track
    fn update_shuffled_queue(&mut self) {
        if self.is_empty() {
            self.shuffled = Vec::new();
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

    /// Removes all songs from the queue except the currently playing one
    pub fn clear_queue(&mut self) {
        let current_song = self.remove(self.current_index());
        self.songs = vec![current_song];
        self.shuffled = vec![0];
        self.ui_update_queue();
        self.ui_close_queue_subpage();
    }

    /// Moves a song in the queue from `from` to `to`
    pub fn reorder(&mut self, from: usize, to: usize) {
        match self.shuffle {
            true => self.shuffled.reorder(from, to),
            false => self.songs.reorder(from, to),
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
            self.ui_close_queue_subpage();
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
        if index <= self.index {
            self.ui_close_queue_subpage();
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
    #[inline]
    #[must_use]
    pub const fn is_first(&self) -> bool {
        self.index == 0
    }

    /// Returns `true` if the current song is last in the queue
    #[inline]
    #[must_use]
    pub const fn is_last(&self) -> bool {
        self.index == self.len() - 1
    }

    /// Returns `true` if there are more tracks in the queue,
    /// or `false` if there is nothing to play afterwrads
    /// Always returns `true` if `repeat` is enabeld
    #[inline]
    #[must_use]
    pub const fn has_next(&self) -> bool {
        self.repeat || !self.is_last()
    }

    /// Returns the total number of songs in the queue
    ///
    /// Note: Do not use to index into `shuffled` when shuffle is disabled
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.songs.len()
    }

    /// Returns `true` if the queue contains no songs
    #[inline]
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
        if shuffle {
            self.update_shuffled_queue();
        }
        self.ui_update_queue();
    }

    /// Returns the current shuffle mode for the queue
    #[inline]
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
        self.ui_update_queue();
    }

    /// Returns the current repeat mode for the queue
    #[inline]
    #[must_use]
    pub const fn get_repeat(&self) -> bool {
        self.repeat
    }

    /// Updates the UI with the current queue shuffle mode setting
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    fn ui_update_shuffle(&self) {
        #[cfg(debug_assertions)]
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
        #[cfg(debug_assertions)]
        println!("ui_update_repeat({})", self.repeat);
        self.ui_tx
            .send(UpdateUI::Repeat(self.repeat))
            .expect(EXP_RX);
    }

    /// Updates the UI with the current queue
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    pub fn ui_update_queue(&mut self) {
        #[cfg(debug_assertions)]
        println!("ui_update_queue()");
        self.ui_tx
            .send(UpdateUI::SetQueue(self.ordered_queue(), self.index))
            .expect(EXP_RX);
        self.last_ui_index = self.index;
    }

    /// Updates the UI with the current queue index
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    pub fn ui_update_queue_index(&mut self) {
        if self.index == self.last_ui_index {
            return;
        }
        #[cfg(debug_assertions)]
        println!("ui_update_queue_index({})", self.index);
        self.ui_tx
            .send(UpdateUI::SetQueueIndex(self.index))
            .expect(EXP_RX);
        self.last_ui_index = self.index;
    }

    /// Closes the UI queue subpage if visible
    ///
    /// # Panics
    /// The function panics if the UI channel receiver is closed
    fn ui_close_queue_subpage(&self) {
        #[cfg(debug_assertions)]
        println!("ui_close_queue_subpage({})", self.index);
        self.ui_tx.send(UpdateUI::CloseQueueSubpage).expect(EXP_RX);
    }

    /// Serializes `self.queue` to a file on disk, or removes
    /// the file if `remember` is `false`
    ///
    /// # Panics
    /// The function panics if `CONFIG_DIR` is unititialized
    #[inline]
    pub fn save_queue(&self, remember: bool, time: Option<u64>) {
        let queue_file = Self::queue_file(CONFIG_DIR.get().expect(EXP_INIT));
        if !remember {
            let _ = fs::remove_file(&queue_file);
            return;
        }
        let contents = self.index.to_string()
            + "\n"
            + &time.map_or_else(|| String::from("-"), |time| time.to_string())
            + "\n"
            + &self.shuffle.to_string()
            + "\n"
            + self
                .songs
                .iter()
                .map(|item| match item {
                    QueueItem::Song(song) => song.info().file_path() + "\n",
                    QueueItem::Stopper(stopper) => match stopper.should_close_player() {
                        false => String::from("Pause\n"),
                        true => String::from("Close Player\n")
                    },
                })
                .collect::<String>()
                .trim();
        match fs::write(&queue_file, contents) {
            Ok(()) => println!("Song queue state successfully written to disk"),
            Err(e) => eprintln!("Problems writing queue state: {e}"),
        }
    }
    /// Saves `self.shuffled` queue to a file on disk, or
    /// removes the file if `remember` is `false`
    ///
    /// # Panics
    /// The function panics if `CONFIG_DIR` is unititialized
    #[inline]
    pub fn save_shuffled_queue(&self, remember: bool) {
        let shuffled_file = Self::shuffled_queue_file(CONFIG_DIR.get().expect(EXP_INIT));
        if !(self.shuffle && remember) {
            let _ = fs::remove_file(&shuffled_file);
            return;
        }
        let contents = self
            .shuffled
            .iter()
            .map(|i| i.to_string() + "\n")
            .collect::<String>();
        match fs::write(&shuffled_file, contents.trim()) {
            Ok(()) => println!("Shuffled song queue successfully written to disk"),
            Err(e) => eprintln!("Problems writing queue state: {e}"),
        }
    }
    /// Starts the initial player queue, in the following order of priority:
    /// - From file arguments passed to the program
    /// - From the remembered `queue` and `shuffled_queue` files on disk
    /// - Using all songs from the library, unless none are available
    ///
    /// # Errors
    /// Function may error if the player or UI channel receiver is closed
    ///
    /// # Panics
    /// The function may panic if `LIBRARY_TX` is uninitialized, or if a
    /// required channel is closed
    pub fn init_queue(
        config_dir: &str,
        library: &Library,
        queue_startup_choice: StartupQueueChoice,
    ) -> Result<(), Box<dyn Error>> {
        let player_tx = &library.player_tx;

        // Load the previous queue if file exists
        if let Ok(queue) = fs::read_to_string(Self::queue_file(config_dir))
            && let mut lines = queue.lines()
            && let Some(Ok(track)) = lines.next().map(str::parse)
            && let Some(time) = lines.next().map(str::parse)
            && let Some(Ok(shuffle)) = lines.next().map(str::parse)
            && let queue =
                library.songs_from_paths(&lines.map(String::from).collect::<Vec<String>>())
            && !queue.is_empty()
        {
            let shuffled = if shuffle {
                fs::read_to_string(Self::shuffled_queue_file(config_dir)).map_or(None, |shuffled| {
                    match shuffled.len() > track {
                        true => Some(
                            shuffled
                                .lines()
                                .filter_map(|i| i.trim().parse().ok())
                                .collect(),
                        ),
                        false => None,
                    }
                })
            } else {
                None
            };
            player_tx.send(PlayerRequest::LoadQueue(queue, shuffled, track))?;
            if let Ok(time) = time {
                player_tx.send(PlayerRequest::SeekToTime(ClockTime::from_mseconds(time)))?;
            }
            return Ok(());
        }

        match queue_startup_choice {
            _ if library.songs.is_empty() => {
                // Maybe open the settings page and focus on the directory options?
                // self.ui_tx.send(UpdateUI::FocusLibrary)?;
                library.ui_tx.send(UpdateUI::OpenSheet(true))?;
            }
            StartupQueueChoice::EmptyQueue => library.ui_tx.send(UpdateUI::OpenSheet(true))?,
            StartupQueueChoice::QueueFromSongs => library.play_all_songs(false)?,
            StartupQueueChoice::QueueFromAlbums => {
                let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
                library_tx.send(LibraryRequest::OnAlbumsSet(Box::new(|library| {
                    library.play_all_albums().unwrap();
                    let _ = library
                        .player_tx
                        .send(PlayerRequest::TogglePlay(Some(false)));
                })))?;
            }
            StartupQueueChoice::QueueFromArtists => {
                let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
                library_tx.send(LibraryRequest::OnArtistsSet(Box::new(|library| {
                    library.play_all_artists().unwrap();
                    let _ = library
                        .player_tx
                        .send(PlayerRequest::TogglePlay(Some(false)));
                })))?;
            }
            StartupQueueChoice::QueueFromSongsShuffled => library.play_all_songs(true)?,
            StartupQueueChoice::QueueFromAlbumsShuffled => {
                let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
                library_tx.send(LibraryRequest::OnAlbumsSet(Box::new(|library| {
                    library.shuffle_all_albums().unwrap();
                    let _ = library
                        .player_tx
                        .send(PlayerRequest::TogglePlay(Some(false)));
                })))?;
            }
            StartupQueueChoice::QueueFromArtistsShuffled => {
                let library_tx = LIBRARY_TX.get().expect(EXP_INIT);
                library_tx.send(LibraryRequest::OnArtistsSet(Box::new(|library| {
                    library.shuffle_all_artists().unwrap();
                    let _ = library
                        .player_tx
                        .send(PlayerRequest::TogglePlay(Some(false)));
                })))?;
            }
            StartupQueueChoice::RestoreQueue => {
                if fs::exists([config_dir, "songs"].concat()).is_ok_and(|exists| exists) {
                    library.ui_tx.send(UpdateUI::OpenSheet(true))?;
                } else {
                    // Load all songs into queue on first launch
                    library.play_all_songs(false)?;
                }
            }
        }
        player_tx.send(PlayerRequest::TogglePlay(Some(false)))?;

        Ok(())
    }

    #[inline]
    fn queue_file(config_dir: &str) -> String {
        [config_dir, "queue"].concat()
    }
    #[inline]
    fn shuffled_queue_file(config_dir: &str) -> String {
        [config_dir, "shuffled_queue"].concat()
    }
}
