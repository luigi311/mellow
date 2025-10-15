use core::error::Error;
use gst::prelude::{ElementExt, ElementExtManual, ObjectExt};
use gst::{ClockTime, SeekFlags, State};
use rand::random_range;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::library::Song;
use crate::ui::UpdateUI;

// TODO: MPRIS support for Gnome Shell media controls

pub enum PlayerRequest {
    /// Play or pause depending on the current state
    PlayOrPause,
    /// Skip to the next song in the queue
    SkipNext,
    /// Skip to beginning or previous song
    SkipPrevious,
    /// Seek to a particular point in the song using a 0 to 1 value
    Seek(f64),
    /// Used internally to signal when song is about to end
    SongEnd,
    /// Refresh local player state
    Update,
}

pub struct Player {
    pub repeat: bool,
    pub shuffle: bool, // TODO: Button to randomize the queue instead of shuffle mode
    song_index: usize,
    queue: Vec<Song>,

    pending_state: Option<State>,
    pending_track: bool,
    pending_track_info: bool,
    end_of_queue: bool,
    tokio_rt: tokio::runtime::Runtime,
    backend: gst::Element,
    bus: gst::Bus,
    ui_tx: tokio_mpsc::Sender<UpdateUI>,
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

        let playbin = gst::ElementFactory::make("playbin3").build()?;
        let bus = playbin.bus().unwrap();

        let (player_tx, rx) = mpsc::sync_channel::<PlayerRequest>(4);
        let (ui_tx, ui_rx) = tokio_mpsc::channel::<UpdateUI>(4);

        Ok((
            Player {
                song_index: 0,
                queue: vec![],
                repeat: false,
                shuffle: false,

                pending_track_info: false,
                pending_track: true,
                pending_state: None,
                end_of_queue: false,
                tokio_rt: tokio::runtime::Runtime::new().map_err(|e| e.to_string())?,
                backend: playbin,
                bus,
                ui_tx: ui_tx.clone(),
                rx,
            },
            player_tx,
            ui_tx,
            ui_rx,
        ))
    }

    /// Main controller loop which handles player requests
    pub fn controller(
        &mut self,
        player_tx: mpsc::SyncSender<PlayerRequest>,
    ) -> Result<(), Box<dyn Error>> {
        self.backend.connect("about-to-finish", false, move |_| {
            player_tx.send(PlayerRequest::SongEnd).unwrap();
            None
        });

        // const SEND_RATE: f64 = 16.0;
        // const SEND_DELAY: Duration = Duration::from_millis((1000.0 / SEND_RATE) as u64);
        // let time_update_timer =
        const IDLE_CHECK_RATE: f64 = 32.0;
        const IDLE_DELAY: Duration = Duration::from_millis((1000.0 / IDLE_CHECK_RATE) as u64);
        loop {
            // TODO: Gracefully handle errors whenever possible
            if let Ok(player_request) = self.rx.try_recv() {
                match player_request {
                    PlayerRequest::SongEnd => self.move_next(),
                    PlayerRequest::PlayOrPause => self.play_or_pause(),
                    PlayerRequest::SkipPrevious => self.skip_prev_or_repeat()?,
                    PlayerRequest::Seek(pos) => self.seek_to_position(pos)?,
                    PlayerRequest::SkipNext => self.skip_next(),
                    PlayerRequest::Update => (),
                }
                self.update()?;
                self.ui_set_state()?;
                continue;
            }

            self.ui_set_time()?;
            thread::sleep(IDLE_DELAY);

            // Reset state after the queue ends
            const END_OF_QUEUE: &[gst::MessageType] =
                &[gst::MessageType::Eos, gst::MessageType::StateChanged];
            if self.end_of_queue && self.bus.pop_filtered(END_OF_QUEUE).is_some() {
                self.flush_gst_messages();
                self.backend.set_state(State::Ready)?;
                let _ = self.backend.state(None);
                self.ui_set_state()?;
                self.pending_track = true;
                self.end_of_queue = false;
                self.update()?;
            }

            // Wait the current track to end, then update the UI
            if self.pending_track || self.pending_track_info {
                if self.pending_track {
                    self.pending_track_info = true;
                    self.pending_track = false;
                    self.flush_gst_messages();
                }

                if self.current_time().unwrap_or_default() < ClockTime::from_seconds(1) {
                    self.queue[self.song_index].assign_info_with_fallback();
                    self.ui_set_song_info()?;

                    self.pending_track_info = false;
                    self.flush_gst_messages();
                }
            }
        }
    }

    /// Manages the playback state
    fn update(&mut self) -> Result<(), Box<dyn Error>> {
        if self.song_index == self.queue.len() {
            self.song_index = 0;
            self.end_of_queue = !self.repeat;
            self.pending_track &= !self.end_of_queue;
        }

        if self.pending_track {
            let file_uri = self.queue[self.song_index].file_uri();
            println!("{file_uri}");
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
            State::Paused => State::Playing,
            State::Ready => State::Playing,
            _ => State::Playing,
        });
    }

    /// Moves to the next track in the queue without flushing the stream
    fn move_next(&mut self) {
        self.pending_track = true;
        self.song_index += 1;
    }

    /// Skips to next track
    fn skip_next(&mut self) {
        self.backend.set_property("instant-uri", true);
        if self.song_index + 1 == self.queue.len() {
            self.request_state(State::Ready);
        }
        self.move_next();
    }

    /// Skips to previous track
    fn skip_prev(&mut self) {
        self.backend.set_property("instant-uri", true);
        self.pending_track = true;
        if self.song_index == 0 {
            if self.repeat {
                self.song_index = self.queue.len() - 1;
            }
            return;
        }
        self.song_index -= 1;
    }

    /// Seeks to the beginning of the current track
    fn repeat_song(&self) -> Result<(), Box<dyn Error>> {
        self.seek_to_time(ClockTime::from_seconds(0))
    }

    /// Skips to previous track or restarts the current one if above the time threshold
    fn skip_prev_or_repeat(&mut self) -> Result<(), Box<dyn Error>> {
        const REPEAT_THRESHOLD: ClockTime = ClockTime::from_seconds(10);
        match self.current_time() {
            Some(time) if time > REPEAT_THRESHOLD => self.repeat_song(),
            _ if (self.song_index == 0 && !self.repeat) => self.repeat_song(),
            _ => Ok(self.skip_prev()),
        }
    }

    /// Seek to a position in the song using a 0 to 1 value
    fn seek_to_position(&self, position: f64) -> Result<(), Box<dyn Error>> {
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_precision_loss)]
        #[allow(clippy::cast_sign_loss)]
        let target_ms = (self
            .backend
            .query_duration::<ClockTime>()
            .unwrap_or_else(|| ClockTime::from_seconds(0))
            .mseconds() as f64
            * position) as u64;
        // println!("Target seek position (ms): {target_ms}");
        self.seek_to_time(ClockTime::from_mseconds(target_ms))
    }

    /// Seek to a particular time in the song
    fn seek_to_time(&self, time: ClockTime) -> Result<(), Box<dyn Error>> {
        // FIX: Hangs when seeking towards the end of song and back again
        match self.backend.current_state() {
            State::Playing | State::Paused | State::Ready => (),
            _ => return Ok(()),
        }
        self.backend
            .seek_simple(SeekFlags::FLUSH | SeekFlags::ACCURATE, time)?;
        Ok(())
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
        println!("ui_set_state()\n");
        let state = state.0.map_or_else(|_| State::Null, |_| state.1);
        self.tokio_rt
            .block_on(async move { tx.send(UpdateUI::PlayerState(state, interactive)).await })
    }

    /// Sends the current song info to the UI receiver
    fn ui_set_song_info(&mut self) -> Result<(), SendError<UpdateUI>> {
        let tx = self.ui_tx.clone();
        let song_info = self.queue[self.song_index].info.take();
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

    /// Replaces the current queue with the provided one
    /// Playback state has to be manually updated
    pub fn new_queue(&mut self, queue: Vec<Song>) {
        self.backend.set_property("instant-uri", true);
        self.pending_track = true;
        self.queue = queue;
    }

    /// Restarts the queue from the beginning
    /// Playback state has to be manually updated
    pub fn restart_queue(&mut self) {
        self.backend.set_property("instant-uri", true);
        self.pending_track = true;
        self.song_index = 0;
    }

    /// Randomizez the order of songs in the queue
    /// Playback state has to be manually updated
    pub fn randomize_queue(&mut self) {
        self.backend.set_property("instant-uri", true);
        self.pending_track = true;
        for i in 0..self.queue.len() {
            let rand_index = random_range(0..self.queue.len());
            self.queue.swap(i, rand_index);
        }
        self.song_index = 0;
    }

    /// Removes all upcomming songs from the queue
    pub fn clear_queue(&mut self) {
        let current_song = self.queue.remove(0);
        self.queue = vec![current_song];
    }

    /// Removes all queued songs after the provided index
    pub fn clear_queue_after_index(&mut self, index: usize) {
        while self.queue.len() > index + 1 {
            self.queue.remove(index + 1);
        }
    }

    /// Clears the GStreamer message queue and prints out errors/warnings/EOS
    fn flush_gst_messages(&self) {
        while let Some(message) = self.bus.pop() {
            match message.type_() {
                gst::MessageType::Error => eprintln!("gstreamer error: {message:?}\n"),
                gst::MessageType::Warning => eprintln!("gstreamer warning: {message:?}\n"),
                gst::MessageType::Eos => eprintln!("gstreamer: Reached end of stream"),
                _ => (),
            }
        }
    }
}
