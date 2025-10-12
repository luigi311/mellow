use core::error::Error;
use gst::prelude::{ElementExt, ElementExtManual, ObjectExt};
use gst::{ClockTime, SeekFlags, State};
use rand::random_range;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::sync::mpsc::error::SendError;

use crate::library::{Song, SongInfo};

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

    // TODO: Transmit time automatically instead
    // Either:
    // - on a fixed interval while playing, or
    // - only on seek or state change, and count time manually in the UI
    /// Send the current time to `ui_rx`
    Tick,
}

pub enum PlayerResponse {
    State(State),
    Time(Option<ClockTime>),
    SongInfo(Option<Box<SongInfo>>),
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
    ui_tx: tokio_mpsc::Sender<PlayerResponse>,
    rx: mpsc::Receiver<PlayerRequest>,
}

// NOTE: Set `GST_DEBUG=3` to debug GStreamer
// https://gstreamer.freedesktop.org/documentation/tutorials/basic/debugging-tools.html

impl Player {
    pub fn init() -> Result<
        (
            Player,
            mpsc::SyncSender<PlayerRequest>,
            tokio_mpsc::Receiver<PlayerResponse>,
        ),
        Box<dyn Error>,
    > {
        gst::init().unwrap();

        let playbin = gst::ElementFactory::make("playbin3").build()?;
        let bus = playbin.bus().unwrap();

        let (player_tx, rx) = mpsc::sync_channel::<PlayerRequest>(4);
        let (ui_tx, ui_rx) = tokio_mpsc::channel::<PlayerResponse>(4);

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
                ui_tx,
                rx,
            },
            player_tx,
            ui_rx,
        ))
    }

    /// Main controller loop which handles player requests
    pub fn controller(
        &mut self,
        player_tx: mpsc::SyncSender<PlayerRequest>,
    ) -> Result<(), Box<dyn Error>> {
        // NOTE: This could be moved into the loop below, but then the
        // refresh times would need to be counted manually
        thread::Builder::new()
            .name("player_timer".to_string())
            .spawn({
                const REFRESH_RATE: f64 = 24.0;
                let player_tx = player_tx.clone();
                move || {
                    loop {
                        #[allow(clippy::cast_sign_loss)]
                        #[allow(clippy::cast_possible_truncation)]
                        let iter_delay = Duration::from_millis((1000.0 / REFRESH_RATE) as u64);
                        thread::sleep(iter_delay);
                        let _ = player_tx.send(PlayerRequest::Tick);
                    }
                }
            })?;

        self.backend.connect("about-to-finish", false, move |_| {
            player_tx.send(PlayerRequest::SongEnd).unwrap();
            None
        });

        // TODO: Gracefully handle errors whenever possible

        loop {
            if let Ok(player_request) = self.rx.try_recv() {
                match player_request {
                    PlayerRequest::SongEnd => self.move_next(),
                    PlayerRequest::PlayOrPause => self.play_or_pause(),
                    PlayerRequest::SkipPrevious => self.skip_prev_or_repeat()?,
                    PlayerRequest::Seek(pos) => self.seek_to_position(pos)?,
                    PlayerRequest::SkipNext => self.skip_next(),
                    PlayerRequest::Update => (),
                    PlayerRequest::Tick => {
                        self.transmit_time()?;
                        continue;
                    }
                }

                self.update()?;
                self.transmit_state()?;
            }

            // Reset state after the queue ends
            if self.end_of_queue {
                if self
                    .bus
                    .pop_filtered(&[gst::MessageType::Eos, gst::MessageType::StateChanged])
                    .is_some()
                {
                    self.clear_gst_msg_queue();
                    self.backend.set_state(State::Ready)?;
                    let _ = self.backend.state(None);
                    self.transmit_state()?;
                    self.pending_track = true;
                    self.end_of_queue = false;
                    self.update()?;
                } else {
                    continue;
                }
            }

            // Wait the current track to end, then update the UI
            if self.pending_track || self.pending_track_info {
                self.pending_track_info = true;
                if self.pending_track {
                    self.clear_gst_msg_queue();
                    self.pending_track = false;
                }

                if self.current_time().unwrap_or_default() < ClockTime::from_seconds(1) {
                    self.queue[self.song_index].assign_info_with_fallback();
                    self.transmit_song_info()?;

                    self.pending_track_info = false;
                    self.clear_gst_msg_queue();
                }
            }

            thread::yield_now();
        }
    }

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

    fn transmit_state(&self) -> Result<(), SendError<PlayerResponse>> {
        let tx = self.ui_tx.clone();
        let state = self.backend.state(None);
        println!("transmit_state()\n");
        let state = state.0.map_or_else(|_| State::Null, |_| state.1);
        self.tokio_rt
            .block_on(async move { tx.send(PlayerResponse::State(state)).await })
    }

    fn transmit_song_info(&mut self) -> Result<(), SendError<PlayerResponse>> {
        let tx = self.ui_tx.clone();
        let song_info = self.queue[self.song_index].info.take();
        println!("transmit_song_info()");
        self.tokio_rt
            .block_on(async move { tx.send(PlayerResponse::SongInfo(song_info)).await })
    }

    fn transmit_time(&self) -> Result<(), SendError<PlayerResponse>> {
        let tx = self.ui_tx.clone();
        let time = self.current_time();
        // println!("transmit_time({time:?})");
        self.tokio_rt
            .block_on(async move { tx.send(PlayerResponse::Time(time)).await })
    }

    // fn transmit_current_state(&self, tx: tokio_mpsc::Sender<PlayerResponse>) {}
    /// Clears the GStreamer message queue and prints out errors/warnings/EOS
    fn clear_gst_msg_queue(&self) {
        while let Some(message) = self.bus.pop() {
            match message.type_() {
                gst::MessageType::Error => eprintln!("gstreamer error: {message:?}\n"),
                gst::MessageType::Warning => eprintln!("gstreamer warning: {message:?}\n"),
                gst::MessageType::Eos => println!("gstreamer: Reached end of stream"),
                _ => (),
            }
        }
    }

    /// Sets player state the next time `update()` is called
    const fn request_state(&mut self, state: State) {
        self.pending_state = Some(state);
    }

    fn current_time(&self) -> Option<ClockTime> {
        self.backend.query_position::<ClockTime>()
    }

    fn song_duration(&self) -> Option<ClockTime> {
        if self.queue.is_empty() {
            return None;
        }
        self.queue[0].info.as_ref().map(|info| info.duration)
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

    fn play_or_pause(&mut self) {
        self.request_state(match self.backend.current_state() {
            State::Playing => State::Paused,
            State::Paused => State::Playing,
            State::Ready => State::Playing,
            _ => State::Playing,
        });
    }

    // FIX: Need to press 3 times to skip back a paused song when over 10 seconds
    // It looks like `current_clock_time()` remains outdated while paused
    fn skip_prev_or_repeat(&mut self) -> Result<(), Box<dyn Error>> {
        let current_time = self.backend.current_clock_time();
        if let Some(time) = current_time
            && (time > ClockTime::from_seconds(10) || (self.song_index == 0 && !self.repeat))
        {
            return self.repeat_song();
        }

        self.skip_prev();
        Ok(())
    }

    fn repeat_song(&self) -> Result<(), Box<dyn Error>> {
        match self.backend.current_state() {
            State::Ready | State::Paused => {
                // Can't seek while paused..?
                self.backend.set_state(State::Playing)?;
            }
            State::Playing => (),
            _ => return Ok(()),
        };
        self.backend.seek_simple(
            SeekFlags::FLUSH | SeekFlags::ACCURATE | SeekFlags::TRICKMODE_NO_AUDIO,
            ClockTime::from_seconds(0),
        )?;
        self.backend.set_state(State::Ready)?;
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

    fn skip_next(&mut self) {
        self.backend.set_property("instant-uri", true);
        if self.song_index + 1 == self.queue.len() {
            self.request_state(State::Ready);
        }
        self.move_next();
    }

    fn move_next(&mut self) {
        self.pending_track = true;
        self.song_index += 1;
    }
}
