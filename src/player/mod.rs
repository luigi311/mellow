use core::error::Error;
use gst::prelude::*;
use gst::{ClockTime, SeekFlags, State};
use std::sync::{OnceLock, mpsc};
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;

use crate::excuses::{EXP_RX, INIT_ERR};
use crate::player::{queue_item::QueueItem, song_queue::SongQueue};
use crate::ui::{UI_TX, UpdateUI};

pub mod queue_item;
pub mod song_queue;

// TODO: MPRIS support for Gnome Shell media controls

pub static PLAYER_TX: OnceLock<mpsc::Sender<PlayerRequest>> = OnceLock::new();
pub enum PlayerRequest {
    /// Refresh local player state
    Update,
    /// Play or pause depending on the current state
    TogglePlay(Option<bool>),
    /// Skip to beginning or previous song
    SkipPrevious,
    /// Skip to the next song in the queue
    SkipNext,
    /// Skip to the specified index in the queue
    SkipTo(usize),
    /// Seek to a particular point in the song using a 0 to 1 value
    Seek(f64),
    /// Stop seeking and resume the player state
    SeekDone,
    /// Load the next song without clearing the stream
    LoadNext,
    /// Signaled from `GStreamer` to load next track before EOS (for gapless playback)
    SongEnd,

    /// Load a new queue
    LoadQueue(Vec<QueueItem>, usize),
    /// Appends multiple items to the current queue
    AppendQueue(Vec<QueueItem>),
    /// Move a queue item from the first argument index to the second
    Reorder(usize, usize),
    /// Inserts an item into the queue
    InsertAt(Box<(usize, QueueItem)>),
    /// Inserts an item into the queue relative to the currently playing index
    InsertRelative(Box<(isize, QueueItem)>),
    /// Remove item at the specified index from the queue
    RemoveAt(usize),

    /// Set the playback volume using a 0 to 1 value
    SetVolume(f64),
    /// Turn the shuffle mode on or off
    SetShuffle(bool),
    /// Turn the repeat mode on or off
    SetRepeat(bool),
    /// Turn gapless playback on or off
    SetGapless(bool),
}

// Required by certain variants
impl std::fmt::Debug for PlayerRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Update => "Update".to_string(),
                Self::TogglePlay(play) => format!("TogglePlay({play:?})",),
                Self::SkipPrevious => "SkipPrevious".to_string(),
                Self::SkipNext => "SkipNext".to_string(),
                Self::SkipTo(index) => format!("SkipTo({index})"),
                Self::Seek(pos) => format!("Seek({pos})"),
                Self::SeekDone => "SeekDone".to_string(),
                Self::LoadNext => "LoadNext".to_string(),
                Self::SongEnd => "SongEnd".to_string(),
                Self::LoadQueue(queue, index) =>
                    format!("LoadQueue((…, {index})): {} items", queue.len()),
                Self::AppendQueue(queue) => format!("AppendQueue(…): {} items", queue.len()),
                Self::Reorder(from, to) => format!("Reorder({from}, {to})"),
                Self::InsertAt(item) => format!("InsertAt({}, …)", item.0),
                Self::InsertRelative(item) => format!("InsertRelative({}, …)", item.0),
                Self::RemoveAt(index) => format!("RemoveAt({index})"),
                Self::SetVolume(volume) => format!("SetVolume({volume})"),
                Self::SetShuffle(shuffle) => format!("SetShuffle({shuffle})"),
                Self::SetRepeat(repeat) => format!("SetRepeat({repeat})"),
                Self::SetGapless(gapless) => format!("SetGapless({gapless})"),
            }
        )
    }
}

pub struct Player {
    pub queue: SongQueue,

    gapless: bool,

    current_state: State,
    pending_state: Option<State>,
    /// Note: When the next song is loaded, `self.queue.index()` returns the next track index
    next_song_loaded: bool,
    seeking: bool,

    backend: gst::Element,
    bus: gst::Bus,
    ui_tx: tokio_mpsc::UnboundedSender<UpdateUI>,
    player_tx: mpsc::Sender<PlayerRequest>,
    rx: mpsc::Receiver<PlayerRequest>,
}

// NOTE: Set `GST_DEBUG=3` to debug GStreamer
// https://gstreamer.freedesktop.org/documentation/tutorials/basic/debugging-tools.html

type PlayerInit = (
    Player,
    mpsc::Sender<PlayerRequest>,             // Player sender
    tokio_mpsc::UnboundedSender<UpdateUI>,   // UI sender
    tokio_mpsc::UnboundedReceiver<UpdateUI>, // UI receiver
);

impl Player {
    /// Returns a tuple of `Player`/`player_tx`/`ui_tx`/`ui_rx`
    /// and initializes `PLAYER_TX`/`UI_TX`
    ///
    /// # Panics
    /// The function panics if initialization fails;
    /// initializing multiple times is not allowed
    #[inline]
    pub fn init() -> PlayerInit {
        gst::init().expect("Could not initialize GStreamer");

        let backend = gst::ElementFactory::make("playbin3")
            .build()
            .expect("Failed to create GStreamer element playbin3");
        let bus = backend.bus().expect(INIT_ERR);

        let (player_tx, rx) = mpsc::channel::<PlayerRequest>();
        let (ui_tx, ui_rx) = tokio_mpsc::unbounded_channel::<UpdateUI>();
        PLAYER_TX
            .set(player_tx.clone())
            .expect("Only one instance of Player is allowed");
        UI_TX
            .set(ui_tx.clone())
            .expect("Cannot initialize UI_TX multiple times");

        (
            Player {
                queue: SongQueue::new(player_tx.clone(), ui_tx.clone()),

                gapless: true,

                current_state: State::Null,
                pending_state: None,
                next_song_loaded: false,
                seeking: false,

                backend,
                bus,
                ui_tx: ui_tx.clone(),
                player_tx: player_tx.clone(),
                rx,
            },
            player_tx,
            ui_tx,
            ui_rx,
        )
    }

    /// Main controller loop which handles player requests
    ///
    /// # Errors
    /// The function may error upon handling a request:
    /// - If a required channel receiver is closed
    /// - Due to an unhandled `GStreamer` error
    ///
    /// # Panics
    /// The function may panic when handling a request
    /// in some cases, such as:
    /// - A `QueueItem` contains a poisoned `Mutex`
    /// - A required channel receiver is closed
    /// - A crash occurs in `GStreamer`
    pub fn controller(&mut self) -> Result<(), Box<dyn Error>> {
        let player_tx = self.player_tx.clone();

        // Required for gapless playback
        self.backend.connect("about-to-finish", false, move |_| {
            // Cannot fail because the receiver is owned by `self`
            let _ = player_tx.send(PlayerRequest::SongEnd);
            None
        });

        loop {
            const UPDATE_RATE: f64 = 60.2; // IDEA: Could be calculated using widget width and track length
            #[allow(clippy::cast_sign_loss)]
            #[allow(clippy::cast_possible_truncation)]
            const UPDATE_INTERVAL: Duration = Duration::from_millis((1000.0 / UPDATE_RATE) as u64);

            self.handle_gst_events();
            let Ok(player_request) = self.rx.recv_timeout(UPDATE_INTERVAL) else {
                self.ui_set_time();
                continue;
            };

            dbg!(&player_request);
            #[allow(clippy::unit_cmp)]
            if match player_request {
                PlayerRequest::Update => true,
                PlayerRequest::TogglePlay(None) => self.play_or_pause() == (),
                PlayerRequest::TogglePlay(Some(play)) => match self.current_state {
                    State::Playing if !play => self.play_or_pause() == (),
                    State::Playing => continue,
                    _ if play => self.play_or_pause() == (),
                    _ => continue,
                },
                PlayerRequest::SkipPrevious => self.skip_prev_or_repeat()? == (),
                PlayerRequest::SkipNext => self.skip_next()? == (),
                PlayerRequest::SkipTo(index) => self.skip_to(index)? == (),
                PlayerRequest::Seek(pos) => self.seek_to_position_paused(pos)? != (),
                PlayerRequest::SeekDone => self.seek_done() == (),
                PlayerRequest::LoadNext if self.seeking => continue,
                PlayerRequest::SongEnd if !self.can_use_gapless() => match self.seeking {
                    true => println!("Ignoring SongEnd while seeking") != (),
                    false => self.request_state(self.current_state) == (),
                },
                PlayerRequest::LoadNext | PlayerRequest::SongEnd => self.move_next(true) == (),

                PlayerRequest::LoadQueue(queue, index) => self.load_queue(queue, index) == (),
                PlayerRequest::AppendQueue(queue) => self.queue.append(&queue) != (),
                PlayerRequest::Reorder(from, to) => self.reorder(from, to) == (),
                PlayerRequest::InsertAt(item) => self.insert_to_queue(item.0, item.1) == (),
                PlayerRequest::InsertRelative(item) => {
                    self.insert_to_queue(
                        match item.0 >= 0 {
                            true => self.queue.index() + item.0 as usize,
                            false => self.queue.index() - -item.0 as usize,
                        },
                        item.1,
                    ) == ()
                }
                PlayerRequest::RemoveAt(index) => {
                    if index == self.queue.index() {
                        if self.next_song_loaded {
                            println!("Removing song which is already loaded");
                            self.unload_gapless();
                            self.queue.remove(index);
                            self.ui_set_state();
                            continue;
                        }
                        self.backend.set_property("instant-uri", true);
                        self.queue.pending_track = true;
                        self.queue.remove(index);
                        true
                    } else {
                        self.queue.remove(index);
                        continue;
                    }
                }

                PlayerRequest::SetVolume(vol) => self.set_volume(vol) != (),
                PlayerRequest::SetShuffle(shuffle) => self.queue.set_shuffle(shuffle) != (),
                PlayerRequest::SetRepeat(repeat) => self.queue.set_repeat(repeat) != (),
                PlayerRequest::SetGapless(gapless) => (self.gapless = gapless) != (),
            } {
                self.update();
                self.ui_set_state();
            }
        }
    }

    /// Manages the playback state
    fn update(&mut self) {
        if self.queue.is_empty() {
            eprintln!("Queue is empty - cannot update player");
            return;
        }

        let file_uri = match self.queue.current() {
            QueueItem::Song(song) => song.lock().unwrap().info().file_uri(),
            QueueItem::Stopper => {
                self.queue.remove_current();
                let _ = self.backend.set_state(State::Null);
                self.request_state(State::Paused);
                return self.update();
            }
        };

        if self.queue.pending_track {
            println!("\n{file_uri}");
            self.backend.set_property("uri", file_uri);
            self.queue.pending_track = false;
            self.next_song_loaded = true;

            if self.current_state == State::Null {
                self.request_state(State::Paused);
            }
        }

        if let Some(state) = self.pending_state.take() {
            match self.backend.set_state(state) {
                Ok(_) => self.current_state = state,
                Err(_) => self.force_skip_track(state),
            }
        }

        // Re-enable gapless playback (for example after track skip)
        self.backend.set_property("instant-uri", false);
    }

    /// Replaces the song queue with `queue` and skips to `index`
    ///
    /// # Panics
    /// The function panics if `index` is out of bounds of `queue`,
    /// except when the `queue` is empty
    fn load_queue(&mut self, queue: Vec<QueueItem>, index: usize) {
        if queue.is_empty() {
            let _ = self.backend.set_state(State::Null);
            self.queue.load_new(queue);
            self.ui_open_playing();
            return;
        }

        self.queue.load_new(queue);
        self.skip_to(index);
    }

    /// Starts or pauses playback depending on state
    fn play_or_pause(&mut self) {
        self.request_state(match self.backend.current_state() {
            State::Playing => State::Paused,
            _ => State::Playing,
        });
    }

    /// Moves to the next track in the queue without flushing the stream
    fn move_next(&mut self, count_played: bool) {
        if count_played {
            // TODO: More advanced play counting (seeking to the end should not count)
            self.queue.current().as_song().info().played();
        }
        self.queue.pending_track = true;
        self.queue.move_next();
    }

    /// Skips to next track
    fn skip_next(&mut self) -> Result<(), gst::StateChangeError> {
        if self.next_song_loaded {
            return self.repeat_song();
        }
        self.backend.set_property("instant-uri", true);
        if !self.queue.get_repeat() && self.queue.is_last() {
            self.request_state(State::Ready);
            return Ok(());
        }
        self.move_next(false);
        Ok(())
    }

    /// Skips to the track in the queue at specified `index`
    fn skip_to(&mut self, index: usize) -> Result<(), gst::StateChangeError> {
        if self.queue.index() == index {
            return self.repeat_song();
        }
        self.backend.set_property("instant-uri", true);
        self.queue.pending_track = true;
        self.queue.set_index(index);
        Ok(())
    }

    /// Skips to previous track
    fn skip_prev(&mut self) {
        self.backend.set_property("instant-uri", true);
        self.queue.pending_track = true;
        self.queue.move_previous();
    }

    /// Seeks to the beginning of the current track
    fn repeat_song(&self) -> Result<(), gst::StateChangeError> {
        self.seek_to_time(ClockTime::ZERO)
    }

    /// Skips to previous track or restarts the current one if above the time threshold
    fn skip_prev_or_repeat(&mut self) -> Result<(), Box<dyn Error>> {
        const REPEAT_THRESHOLD: ClockTime = ClockTime::from_seconds(10);
        match self.current_time() {
            Some(time)
                if !self.next_song_loaded
                    && (time > REPEAT_THRESHOLD
                        || (self.queue.is_first() && !self.queue.get_repeat())) =>
            {
                self.repeat_song()?;
            }
            _ => self.skip_prev(),
        }
        Ok(())
    }

    /// Seek to a position in the song using a 0 to 1 value
    fn seek_to_position(&self, position: f64) -> Result<(), gst::StateChangeError> {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_precision_loss)]
        #[allow(clippy::cast_sign_loss)]
        let target_ms = ((self.backend.query_duration::<ClockTime>())
            .unwrap_or_default()
            .mseconds() as f64
            * position) as u64;
        // println!("Target seek position (ms): {target_ms}");
        self.seek_to_time(ClockTime::from_mseconds(target_ms))
    }

    /// Seek to a particular time in the song
    fn seek_to_time(&self, time: ClockTime) -> Result<(), gst::StateChangeError> {
        match (self.backend).seek_simple(SeekFlags::FLUSH | SeekFlags::ACCURATE, time) {
            Ok(()) => {
                self.backend.state(None).0?;
                self.ui_set_time();
            }
            Err(e) => eprintln!("{e}"),
        }
        Ok(())
    }

    /// Seek to a position in the song using a 0 to 1 value
    /// Remember to call `seek_done()` to resume playback
    fn seek_to_position_paused(&mut self, position: f64) -> Result<(), gst::StateChangeError> {
        self.begin_seek_paused()?;
        self.seek_to_position(position)
    }

    /// Prepare the palyer for interactive seeking in paused state
    /// Remember to call `seek_done()` to resume playback
    fn begin_seek_paused(&mut self) -> Result<(), gst::StateChangeError> {
        // If next track is already loaded, move back to the current one
        if self.next_song_loaded {
            println!("Gapless transition interrupted by seek request");
            self.backend.set_state(State::Null)?;
            self.request_state(self.current_state);
            let _ = self.backend.state(None); // Wait for backend state
            self.next_song_loaded = false;
            self.skip_prev();
            self.update();
            let _ = self.backend.state(None); // Wait for backend state
            self.queue.current().as_song().info().deduct_played();
        }

        match self.backend.current_state() {
            State::Playing => self.backend.set_state(State::Paused).map(|_| ())?,
            State::Paused => (),
            _ => return Ok(()),
        }

        self.seeking = true;
        let _ = self.backend.state(None); // Wait for backend state
        Ok(())
    }

    /// Call to resume the player state when done seeking
    fn seek_done(&mut self) {
        self.seeking = false;
        let (pos, dur) = (
            self.backend.query_position::<ClockTime>(),
            (self.backend.query_duration::<ClockTime>()).map(ClockTime::mseconds),
        );

        if let (Some(pos), Some(dur)) = (pos, dur)
            && dur.saturating_sub(pos.mseconds()) != 0
        {
            // Reset state to re-enable missed `GStreamer` callbacks
            let _ = self.backend.set_state(State::Null);
            let _ = self.backend.state(None); // Wait for backend state
            let _ = self.backend.set_state(State::Paused);
            let _ = self.backend.state(None); // Wait for backend state

            // Seek one final time after state reset
            let _ = self.seek_to_time(pos);
        } else {
            // Skip the current song if the song has ended
            // or the playback time/duration cannot be determined
            self.player_tx.send(PlayerRequest::SkipNext).expect(EXP_RX);
        }

        self.request_state(self.current_state);
    }

    /// Unloads the gaplessly loaded track by restarting the stream
    ///
    /// Note that this might cause an audible stutter, so use it sparingly
    pub fn unload_gapless(&mut self) {
        println!("---- Unloading gapless track ----");
        let Some(pos) = self.backend.query_position::<ClockTime>() else {
            eprintln!("Could not determine playback time, skipping...");
            let _ = self.player_tx.send(PlayerRequest::SkipNext);
            return;
        };

        let _ = self.backend.set_state(State::Null);
        self.request_state(self.current_state);
        let _ = self.backend.state(None); // Wait for backend state
        self.next_song_loaded = false;
        self.skip_prev();
        self.update();
        let _ = self.backend.state(None); // Wait for backend state
        self.queue.current().as_song().info().deduct_played();

        // Seek to the same time the player was at before, or skip the song
        if self.seek_to_time(pos).is_err() {
            self.queue.current().map(|mut song| song.info().played());
            let _ = self.player_tx.send(PlayerRequest::SkipNext);
        }
    }

    /// Sets the playback volume
    fn set_volume(&self, volume: f64) {
        self.backend.set_property("volume", volume);
    }

    fn reorder(&mut self, from: usize, to: usize) {
        if self.next_song_loaded
            && (from == self.queue.index() - 1
                || from == self.queue.index()
                || (from < to && to == self.queue.index() - 1)
                || (from > to && to == self.queue.index()))
        {
            self.unload_gapless();
        }
        self.queue.reorder(from, to);
    }

    /// Inserts a `QueueItem` into the current queue at the specified `index`
    fn insert_to_queue(&mut self, index: usize, item: QueueItem) {
        if self.next_song_loaded && index == self.queue.index() {
            self.unload_gapless();
        }
        self.queue.insert(index, item);
    }

    /// Sets player state the next time `update()` is called
    const fn request_state(&mut self, state: State) {
        self.pending_state = Some(state);
    }

    /// Current playback time in the song
    fn current_time(&self) -> Option<ClockTime> {
        self.backend.query_position::<ClockTime>()
    }

    /// Retruns `true` if a gapless transition is appropriate for the
    /// current state. Always returns `false` if gapless mode is disabled
    fn can_use_gapless(&self) -> bool {
        self.gapless
            && !self.seeking
            && !self.queue.next().is_some_and(QueueItem::is_stopper)
            && (!self.queue.is_last() || self.queue.get_repeat())
    }

    /// Sends the current state to the UI receiver
    fn ui_set_state(&self) {
        let state = self.backend.state(None);
        let interactive = !self.queue.is_empty();
        let playing = state.0.is_ok() && matches!(state.1, State::Playing);
        println!("ui_set_state(playing: {playing}, interactive: {interactive})");
        self.ui_tx
            .send(UpdateUI::PlayerState(playing, interactive))
            .expect(EXP_RX);
    }

    /// Sends the current song info to the UI receiver
    fn ui_update_song_info(&self) {
        println!("ui_update_song_info()");
        self.ui_tx.send(UpdateUI::SongInfo).expect(EXP_RX);
    }

    /// Sends the current playback time to the UI receiver
    fn ui_set_time(&self) {
        let time = self.current_time();
        // println!("ui_set_time({time:?})");
        self.ui_tx.send(UpdateUI::PlayerTime(time)).expect(EXP_RX);
    }

    /// Requests the UI to open the music library
    fn ui_open_playing(&self) {
        self.ui_tx.send(UpdateUI::FocusPlaying).expect(EXP_RX);
        self.ui_tx.send(UpdateUI::OpenSheet(true)).expect(EXP_RX);
    }

    /// Handles `GStreamer` events and empties the message queue
    fn handle_gst_events(&mut self) {
        while let Some(message) = self.bus.pop() {
            match message.type_() {
                gst::MessageType::StreamStart => {
                    println!("Song started");
                    self.queue.ui_update_queue_index();
                    self.ui_update_song_info();
                    self.next_song_loaded = false;
                }
                gst::MessageType::Eos if self.seeking => {
                    println!("EOS ignored while seeking");
                }
                gst::MessageType::Eos => {
                    if self.queue.has_next() {
                        println!("Moving to next track due to end of stream");
                        self.request_state(State::Playing);
                        self.player_tx.send(PlayerRequest::LoadNext).expect(EXP_RX);
                    } else {
                        println!("Stopping player due to end of queue");
                        self.queue.current().as_song().info().played();
                        self.request_state(State::Null);
                        self.queue.pending_track = true;
                        self.update();
                        self.ui_set_state();
                    }
                }
                gst::MessageType::Warning => eprintln!("gstreamer warning: {message:?}\n"),
                gst::MessageType::Error => {
                    let dbg = format!("{message:?}");
                    eprintln!("gstreamer error: {dbg}\n");

                    if dbg.contains(&self.queue.current().as_song().info().file_uri()) {
                        self.force_skip_track(self.current_state);
                    }
                }
                _ => (),
            }
        }
    }

    /// Removes the current track from queue and resets player state
    /// Should only be used for error handling
    fn force_skip_track(&mut self, new_state: State) {
        // TODO: Display a toast informing the user of the issue
        eprintln!("Skipping song due to an issue");
        self.backend.set_state(State::Null).unwrap();
        self.queue.remove_current();
        // self.move_next();
        self.request_state(new_state);
        self.player_tx.send(PlayerRequest::Update).expect(EXP_RX);
    }
}
