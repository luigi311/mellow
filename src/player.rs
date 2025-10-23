use core::error::Error;
use gst::prelude::{ElementExt, ElementExtManual, ObjectExt};
use gst::{ClockTime, SeekFlags, State};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::player::song_queue::{QueueItem, SongQueue};
use crate::ui::UpdateUI;

pub mod song_queue;

// TODO: MPRIS support for Gnome Shell media controls

#[derive(Debug)]
pub enum PlayerRequest {
    /// Play or pause depending on the current state
    PlayOrPause,
    /// Skip to beginning or previous song
    SkipPrevious,
    /// Seek to a particular point in the song using a 0 to 1 value
    Seek(f64),
    /// Skip to the next song in the queue
    SkipNext,
    /// Loads the next song without clearing the stream
    /// (does nothing if `gapless` is turned off)
    LoadNext,
    /// Refresh local player state
    Update,

    /// Set the playback volume using a 0 to 1 value
    SetVolume(f64),
    /// Turn the shuffle mode on or off
    SetShuffle(bool),
    /// Turn the repeat mode on or off
    SetRepeat(bool),
    /// Turn gapless playback on or off
    SetGapless(bool),

    /// Used internally by `SongQueue`
    SetInstantURI(bool),
}

pub struct Player {
    pub queue: SongQueue,

    gapless: bool,

    pending_state: Option<State>,
    pending_track_info: bool,
    backend: gst::Element,
    bus: gst::Bus,
    tokio_rt: tokio::runtime::Runtime,
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

        let tokio_rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        let (player_tx, rx) = mpsc::sync_channel::<PlayerRequest>(4);
        let (ui_tx, ui_rx) = tokio_mpsc::channel::<UpdateUI>(4);

        Ok((
            Player {
                queue: SongQueue::new(player_tx.clone()),

                gapless: true,

                pending_track_info: false,
                pending_state: None,
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
        let player_tx = self.player_tx.clone();
        self.backend.connect("about-to-finish", false, move |_| {
            player_tx.send(PlayerRequest::LoadNext).unwrap();
            None
        });

        // const SEND_RATE: f64 = 16.0;
        // const SEND_DELAY: Duration = Duration::from_millis((1000.0 / SEND_RATE) as u64);
        // let time_update_timer =
        const IDLE_CHECK_RATE: f64 = 60.2;
        const IDLE_DELAY: Duration = Duration::from_millis((1000.0 / IDLE_CHECK_RATE) as u64);
        loop {
            // TODO: Gracefully handle errors whenever possible

            // FIX: Panic on unsupproted file
            // Create a 0 byte music file and try to play it
            // Expected behavior: prints an error and skips the song
            // Actual behavior: the program panics

            // FIX: Panic on missing file
            // Play music and delete or rename the next song in the queue
            // Expected behavior: prints an error and skips the song
            // Actual behavior: the program panics

            if let Ok(player_request) = self.rx.try_recv() {
                dbg!(&player_request);
                match player_request {
                    PlayerRequest::Update => (),
                    PlayerRequest::PlayOrPause => self.play_or_pause(),
                    PlayerRequest::SkipPrevious => self.skip_prev_or_repeat()?,
                    PlayerRequest::Seek(pos) => self.seek_to_position(pos)?,
                    PlayerRequest::SkipNext => self.skip_next(),
                    PlayerRequest::LoadNext => {
                        if !self.gapless {
                            continue;
                        }
                        self.move_next();
                    }

                    no_update => {
                        match no_update {
                            PlayerRequest::SetVolume(vol) => self.set_volume(vol),
                            PlayerRequest::SetShuffle(shuffle) => self.queue.set_shuffle(shuffle),
                            PlayerRequest::SetRepeat(repeat) => self.queue.repeat = repeat,
                            PlayerRequest::SetGapless(gapless) => self.gapless = gapless,
                            PlayerRequest::SetInstantURI(instant_uri) => {
                                self.backend.set_property("instant-uri", instant_uri);
                            }
                            request => panic!("Unhandled player request: {request:?}"),
                        }
                        continue;
                    }
                }
                self.update()?;
                self.ui_set_state()?;
                continue;
            }

            self.ui_set_time()?;
            thread::sleep(IDLE_DELAY);

            // Reset state after the queue ends
            const EOQ_FILTERS: &[gst::MessageType] =
                &[gst::MessageType::Eos, gst::MessageType::StateChanged];
            if self.queue.end_of_queue && self.bus.pop_filtered(EOQ_FILTERS).is_some() {
                self.handle_gst_messages();
                self.backend.set_state(State::Ready)?;
                let _ = self.backend.state(None);
                self.ui_set_state()?;
                self.queue.pending_track = true;
                self.queue.end_of_queue = false;
                self.update()?;
            }

            // Wait the current track to end, then update the UI
            if self.queue.pending_track || self.pending_track_info {
                if self.queue.pending_track {
                    self.pending_track_info = true;
                    self.queue.pending_track = false;
                }

                if self.current_time().unwrap_or_default() < ClockTime::from_seconds(1) {
                    self.queue
                        .get_current()
                        .as_mut_song()
                        .assign_info_with_fallback();
                    self.ui_set_song_info()?;
                    self.pending_track_info = false;
                }
            }

            self.handle_gst_messages();
        }
    }

    /// Manages the playback state
    fn update(&mut self) -> Result<(), Box<dyn Error>> {
        let file_uri = match self.queue.get_current() {
            QueueItem::Song(song) => song.file_uri(),
            QueueItem::Stopper => {
                self.queue.remove_current();
                self.request_state(State::Paused);
                return self.update();
            }
        };

        if self.queue.pending_track {
            println!("\n{file_uri}");
            self.backend.set_property("uri", file_uri);
        }

        if let Some(state) = self.pending_state.take() {
            self.backend.set_state(state)?;
        }

        // Re-enable gapless playback (for example after track skip)
        self.backend.set_property("instant-uri", false);

        Ok(())
    }

    /// Starts or pauses playback depending on state
    fn play_or_pause(&mut self) {
        self.request_state(match self.backend.current_state() {
            State::Playing => State::Paused,
            _ => State::Playing,
        });
    }

    /// Moves to the next track in the queue without flushing the stream
    fn move_next(&mut self) {
        self.queue.pending_track = true;
        self.queue.next();
    }

    /// Skips to next track
    fn skip_next(&mut self) {
        self.backend.set_property("instant-uri", true);
        if !self.queue.repeat && self.queue.is_last() {
            self.request_state(State::Ready);
        }
        self.move_next();
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
                if (time > REPEAT_THRESHOLD || (self.queue.is_first() && !self.queue.repeat)) =>
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
        // FIX: Hangs when seeking to song end
        match self.backend.current_state() {
            State::Playing | State::Paused | State::Ready => (),
            _ => return Ok(()),
        }
        self.backend
            .seek_simple(SeekFlags::FLUSH | SeekFlags::ACCURATE, time)?;
        Ok(())
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
    fn ui_set_song_info(&mut self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        let song_info = self.queue.get_current().as_mut_song().info.take();
        println!("ui_set_song_info()");
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::SongInfo(song_info)).await })
    }

    /// Sends the current playback time to the UI receiver
    fn ui_set_time(&self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        let time = self.current_time();
        // println!("ui_set_time({time:?})");
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::PlayerTime(time)).await })
    }

    /// Clears and hadles the GStreamer message queue
    fn handle_gst_messages(&self) {
        while let Some(message) = self.bus.pop() {
            match message.type_() {
                gst::MessageType::Error => eprintln!("gstreamer error: {message:?}\n"),
                gst::MessageType::Warning => eprintln!("gstreamer warning: {message:?}\n"),
                gst::MessageType::Eos => {
                    println!("gstreamer: Reached end of stream");
                    self.player_tx.send(PlayerRequest::SkipNext).unwrap();
                }
                _ => (),
            }
        }
    }
}
