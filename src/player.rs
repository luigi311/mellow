use core::error::Error;
use gst::prelude::*;
use gst::{ClockTime, SeekFlags, State};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::player::song_queue::{QueueItem, SongQueue};
use crate::ui::UpdateUI;

pub mod song_queue;

// TODO: MPRIS support for Gnome Shell media controls

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
    /// Signaled from GStreamer to load next track before EOS (for gapless playback)
    SongEnd,

    /// Load a new queue
    LoadQueue(Vec<QueueItem>),
    /// Inserts an item into the queue
    InsertAt(Box<(QueueItem, usize)>),
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

// Required due to `PlayerRequest::LoadQueue`
impl std::fmt::Debug for PlayerRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::LoadQueue(queue) => {
                    format!("LoadQueue(…): {} items", queue.len())
                }
                Self::Update => "Update".to_string(),
                Self::TogglePlay(play) => format!("TogglePlay({play:?})",),
                Self::SkipPrevious => "SkipPrevious".to_string(),
                Self::SkipNext => "SkipNext".to_string(),
                Self::SkipTo(index) => format!("SkipTo({index})"),
                Self::Seek(pos) => format!("Seek({pos})"),
                Self::SeekDone => "SeekDone".to_string(),
                Self::LoadNext => "LoadNext".to_string(),
                Self::SongEnd => "SongEnd".to_string(),
                Self::InsertAt(item) => format!("InsertAt(…, {})", item.1),
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
    next_song_loaded: bool,
    seeking: bool,

    backend: gst::Element,
    bus: gst::Bus,
    tokio_rt: Arc<tokio::runtime::Runtime>,
    ui_tx: tokio_mpsc::Sender<UpdateUI>,
    player_tx: mpsc::SyncSender<PlayerRequest>,
    rx: mpsc::Receiver<PlayerRequest>,
}

// NOTE: Set `GST_DEBUG=3` to debug GStreamer
// https://gstreamer.freedesktop.org/documentation/tutorials/basic/debugging-tools.html

impl Player {
    /// Returns a tuple of a new `Player` instance, a sender for player controls,
    /// and a sender and receiver for the UI
    pub fn init() -> Result<
        (
            Player,
            mpsc::SyncSender<PlayerRequest>,
            tokio_mpsc::Sender<UpdateUI>,
            tokio_mpsc::Receiver<UpdateUI>,
        ),
        Box<dyn Error>,
    > {
        gst::init().unwrap();

        let backend = gst::ElementFactory::make("playbin3").build()?;
        let bus = backend.bus().unwrap();

        let tokio_rt = Arc::new(tokio::runtime::Runtime::new().map_err(|e| e.to_string())?);
        let (player_tx, rx) = mpsc::sync_channel::<PlayerRequest>(4);
        let (ui_tx, ui_rx) = tokio_mpsc::channel::<UpdateUI>(4);

        Ok((
            Player {
                queue: SongQueue::new(player_tx.clone(), ui_tx.clone(), Arc::clone(&tokio_rt)),

                gapless: true,

                current_state: State::Null,
                pending_state: None,
                next_song_loaded: false,
                seeking: false,

                backend,
                bus,
                tokio_rt,
                ui_tx: ui_tx.clone(),
                player_tx: player_tx.clone(),
                rx,
            },
            player_tx,
            ui_tx,
            ui_rx,
        ))
    }

    /// Main controller loop which handles player requests
    pub fn controller(&mut self) -> Result<(), Box<dyn Error>> {
        const LOOP_RATE: f64 = 60.2;
        const LOOP_DELAY: Duration = Duration::from_millis((1000.0 / LOOP_RATE) as u64);

        // Enable gapless playback
        let player_tx = self.player_tx.clone();
        self.backend.connect("about-to-finish", false, move |_| {
            player_tx.send(PlayerRequest::SongEnd).unwrap();
            None
        });

        loop {
            let Ok(player_request) = self.rx.try_recv() else {
                self.ui_set_time()?;

                thread::sleep(LOOP_DELAY);
                self.handle_gst_events();
                continue;
            };

            dbg!(&player_request);
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
                PlayerRequest::SkipNext => self.skip_next() == (),
                PlayerRequest::SkipTo(index) => self.skip_to(index) == (),
                PlayerRequest::Seek(pos) => self.seek_to_position_paused(pos)? != (),
                PlayerRequest::SeekDone => self.seek_done() == (),
                PlayerRequest::LoadNext if self.seeking => false,
                PlayerRequest::SongEnd if !self.can_use_gapless() => {
                    self.request_state(self.current_state);
                    true
                }
                PlayerRequest::LoadNext | PlayerRequest::SongEnd => self.move_next() == (),

                PlayerRequest::LoadQueue(queue) => self.queue.load_new(queue)? != (),
                PlayerRequest::InsertAt(item) => self.queue.insert(item.1, item.0).map(|_| true)?,
                PlayerRequest::RemoveAt(index) => {
                    self.queue.remove(index);
                    if index == self.queue.index() {
                        self.backend.set_property("instant-uri", true);
                        self.queue.pending_track = true;
                        true
                    } else {
                        continue;
                    }
                }

                PlayerRequest::SetVolume(vol) => self.set_volume(vol) != (),
                PlayerRequest::SetShuffle(shuffle) => self.queue.set_shuffle(shuffle)? != (),
                PlayerRequest::SetRepeat(repeat) => self.queue.set_repeat(repeat)? != (),
                PlayerRequest::SetGapless(gapless) => (self.gapless = gapless) != (),
            } {
                self.update();
                self.ui_set_state()?;
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

    /// Starts or pauses playback depending on state
    fn play_or_pause(&mut self) {
        self.request_state(match self.backend.current_state() {
            State::Playing => State::Paused,
            _ => State::Playing,
        });
    }

    /// Moves to the next track in the queue without flushing the stream
    const fn move_next(&mut self) {
        self.queue.pending_track = true;
        self.queue.move_next();
    }

    /// Skips to next track
    fn skip_next(&mut self) {
        self.backend.set_property("instant-uri", true);
        if !self.queue.get_repeat() && self.queue.is_last() {
            self.request_state(State::Ready);
            return;
        }
        self.move_next();
    }

    /// Skips to the track in the queue at specified `index`
    fn skip_to(&mut self, index: usize) {
        self.backend.set_property("instant-uri", true);
        self.queue.pending_track = true;
        self.queue.set_index(index);
    }

    /// Skips to previous track
    fn skip_prev(&mut self) {
        self.backend.set_property("instant-uri", true);
        self.queue.pending_track = true;
        self.queue.previous();
    }

    /// Seeks to the beginning of the current track
    fn repeat_song(&self) -> Result<(), Box<dyn Error>> {
        self.seek_to_time(ClockTime::default())
    }

    /// Skips to previous track or restarts the current one if above the time threshold
    fn skip_prev_or_repeat(&mut self) -> Result<(), Box<dyn Error>> {
        const REPEAT_THRESHOLD: ClockTime = ClockTime::from_seconds(10);
        match self.current_time() {
            Some(time)
                if (time > REPEAT_THRESHOLD
                    || (self.queue.is_first() && !self.queue.get_repeat())) =>
            {
                self.repeat_song()?;
            }
            _ => self.skip_prev(),
        }
        Ok(())
    }

    /// Seek to a position in the song using a 0 to 1 value
    fn seek_to_position(&self, position: f64) -> Result<(), Box<dyn Error>> {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_precision_loss)]
        #[allow(clippy::cast_sign_loss)]
        let target_ms = (self
            .backend
            .query_duration::<ClockTime>()
            .unwrap_or_default()
            .mseconds() as f64
            * position) as u64;
        // println!("Target seek position (ms): {target_ms}");
        self.seek_to_time(ClockTime::from_mseconds(target_ms))
    }

    /// Seek to a particular time in the song
    fn seek_to_time(&self, time: ClockTime) -> Result<(), Box<dyn Error>> {
        match self
            .backend
            .seek_simple(SeekFlags::FLUSH | SeekFlags::ACCURATE, time)
        {
            Ok(()) => self.backend.state(None).0.map(|_| ())?,
            Err(e) => eprintln!("{e}"),
        }
        Ok(())
    }

    /// Seek to a position in the song using a 0 to 1 value
    /// Remember to call `seek_done()` to resume playback
    fn seek_to_position_paused(&mut self, position: f64) -> Result<(), Box<dyn Error>> {
        self.begin_seek_paused()?;
        self.seek_to_position(position)
    }

    // /// Seek to a particular time in the song
    // /// Remember to call `seek_done()` to resume playback
    // fn seek_to_time_paused(&mut self, time: ClockTime) -> Result<(), Box<dyn Error>> {
    //     self.begin_seek_paused()?;
    //     self.seek_to_time(time)
    // }

    /// Prepare the palyer for interactive seeking in paused state
    /// Remember to call `seek_done()` to resume playback
    fn begin_seek_paused(&mut self) -> Result<(), gst::StateChangeError> {
        self.seeking = true;

        // If next track is already loaded, move back to the current one
        if self.next_song_loaded {
            // FIX: Seeking after next song is loaded causes playback issues
            self.queue.pending_track = true;
            self.queue.set_index(self.queue.index() - 1);
            self.backend.set_property("instant-uri", true);
            self.update();
            self.next_song_loaded = false;
        }

        match self.backend.current_state() {
            State::Playing => self.backend.set_state(State::Paused).map(|_| ())?,
            State::Paused => (),
            _ => return Ok(()),
        }
        Ok(())
    }

    /// Call to resume the player state when done seeking
    fn seek_done(&mut self) {
        self.seeking = false;
        if self.backend.current_state() == State::Null {
            self.player_tx.send(PlayerRequest::LoadNext).unwrap();
        }
        self.request_state(self.current_state);
    }

    fn set_volume(&self, volume: f64) {
        self.backend.set_property("volume", volume);
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
    fn ui_set_state(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        let state = self.backend.state(None);
        let interactive = !self.queue.is_empty();
        println!("ui_set_state()");
        let state = state.0.map_or_else(|_| State::Null, |_| state.1);
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::PlayerState(state, interactive)).await })
    }

    /// Sends the current song info to the UI receiver
    fn ui_update_song_info(&mut self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        println!("ui_update_song_info()");
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::SongInfo).await })
    }

    /// Sends the current playback time to the UI receiver
    fn ui_set_time(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        let time = self.current_time();
        // println!("ui_set_time({time:?})");
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::PlayerTime(time)).await })
    }

    /// Handles `GStreamer` events and empties the message queue
    fn handle_gst_events(&mut self) {
        while let Some(message) = self.bus.pop() {
            match message.type_() {
                gst::MessageType::Error => {
                    let dbg = format!("{message:?}");
                    eprintln!("gstreamer error: {dbg}\n");

                    if dbg.contains(&self.queue.current().as_song().info().file_uri()) {
                        self.force_skip_track(self.current_state);
                    }
                }
                gst::MessageType::Warning => eprintln!("gstreamer warning: {message:?}\n"),
                gst::MessageType::StreamStart => {
                    println!("Song started");
                    self.queue.ui_update_queue_index().unwrap();
                    self.ui_update_song_info().unwrap();
                    self.next_song_loaded = false;
                }
                gst::MessageType::Eos if self.seeking => {
                    println!("EOS ignored while seeking");
                    self.request_state(self.current_state);
                    self.player_tx.send(PlayerRequest::Update).unwrap();
                }
                gst::MessageType::Eos => {
                    println!("Reached end of stream");

                    while self.bus.pop_filtered(&[gst::MessageType::Eos]).is_some() {
                        eprintln!("Warning: EOS received multiple times");
                    }

                    if !self.queue.has_next() {
                        println!("End of queue");
                        self.backend.set_state(State::Ready).unwrap();
                        let _ = self.backend.state(None);
                        self.ui_set_state().unwrap();
                        self.queue.pending_track = true;
                        self.update();
                    }

                    if self.current_state == State::Playing {
                        println!("Moving to next track due to EOS");
                        self.request_state(State::Playing);
                        self.player_tx.send(PlayerRequest::LoadNext).unwrap();
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
        self.player_tx.send(PlayerRequest::Update).unwrap();
    }
}
