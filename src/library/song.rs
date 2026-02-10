use core::error::Error;
use gio::prelude::*;
use gst::ClockTime;
use gtk::{gdk, gio, glib};
use std::backtrace::Backtrace;
use std::mem;
use std::sync::{Arc, Mutex};

use lofty::file::TaggedFile;
use lofty::prelude::*;
use lofty::probe::Probe;

use crate::library::album::SharedAlbum;
use crate::{deserialize, serialize};

pub type SharedSong = Arc<Mutex<Song>>;
pub trait SharedSongExt {
    fn from_file(file: gio::File) -> SharedSong;
    fn from_path(path: &str) -> SharedSong;
    fn load_detailed_info(&self) -> Result<(), Box<dyn Error>>;
    fn try_load_detailed_info(&self) -> Result<(), Box<dyn Error>>;
    fn detailed_info_from_tags(&self, tagged: &TaggedFile) -> Option<DetailedSongInfo>;
}
impl SharedSongExt for SharedSong {
    /// Constructs a new `SharedSong` from a `gio::File`
    #[inline]
    fn from_file(file: gio::File) -> SharedSong {
        Arc::new(Mutex::new(Song::from_file(file)))
    }
    /// Constructs a new `SharedSong` from a file path
    #[inline]
    fn from_path(path: &str) -> SharedSong {
        Arc::new(Mutex::new(Song::from_path(path)))
    }

    // NOTE: This loading solution is not ideal, because it doesn't provide
    // a good way to coordinate status with `SongInfoLoader`. It would be
    // possible to remove this implementation and use the loader by using
    // individual mutexes for each info field and returning their guards,
    // but then `album` will need to be wrapped in another `Mutex` as well.

    /// Loads detailed song info so it is ready to be used later.
    /// Does nothing if info is already loading.
    #[inline]
    fn load_detailed_info(&self) -> Result<(), Box<dyn Error>> {
        let mut locked = self.lock().unwrap();
        if !locked.detailed_info.can_load() {
            return Ok(());
        }
        locked.detailed_info = LoadState::Loading;
        let tagged = Probe::open(locked.file.path().unwrap())?.read()?;
        drop(locked);
        let info = self.detailed_info_from_tags(&tagged);
        self.lock().unwrap().detailed_info = LoadState::Loaded(info.unwrap());
        Ok(())
    }
    /// Loads detailed song info so it is ready to be used later,
    /// Does nothing if info is already loading, or if the `Mutex`
    /// is currently locked.
    #[inline]
    fn try_load_detailed_info(&self) -> Result<(), Box<dyn Error>> {
        let Ok(mut locked) = self.try_lock() else {
            return Ok(());
        };
        if !locked.detailed_info.can_load() {
            return Ok(());
        }
        locked.detailed_info = LoadState::Loading;
        let tagged = Probe::open(locked.file.path().unwrap())?.read()?;
        drop(locked);
        let info = self.detailed_info_from_tags(&tagged);
        self.lock().unwrap().detailed_info = LoadState::Loaded(info.unwrap());
        Ok(())
    }
    /// Loads the detailed song info from `tagged`
    #[inline]
    fn detailed_info_from_tags(&self, tagged: &TaggedFile) -> Option<DetailedSongInfo> {
        match SongInfoLoader::load_tags_detailed(tagged) {
            Ok(result) => result,
            Err(e) => {
                eprintln!(
                    "Problem loading tags (detailed): {:?}: {e}",
                    self.lock().unwrap().file.path().unwrap_or_default()
                );
                Some(DetailedSongInfo {
                    lyrics: String::new(),
                    artwork: None,
                })
            }
        }
    }
}

#[derive(Clone)]
pub struct Song {
    pub album: Option<SharedAlbum>,
    file: gio::File,
    info: Option<SongInfo>,
    user_info: UserSongInfo,
    detailed_info: LoadState<DetailedSongInfo>,
}

#[derive(Clone)]
pub struct SongInfo {
    pub title: String,
    pub album: String,
    pub artist: String,
    pub album_artist: String,
    pub track: u32,
    pub disc: u32,
    pub year: u16,
    pub duration: ClockTime,
}

impl Default for SongInfo {
    #[inline]
    fn default() -> Self {
        SongInfo {
            title: String::new(),
            album: String::new(),
            artist: String::new(),
            album_artist: String::new(),
            track: 0,
            disc: 1,
            year: 0,
            duration: ClockTime::ZERO,
        }
    }
}

impl PartialEq for SongInfo {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.album == other.album
            && self.artist == other.artist
            && self.track == other.track
    }
}

#[derive(Clone, Debug)]
pub struct UserSongInfo {
    pub modified: i64,
    pub play_count: usize,
    pub rating: u8,
}

impl UserSongInfo {
    #[must_use]
    pub const fn default() -> Self {
        Self {
            modified: 0,
            play_count: 0,
            rating: 0,
        }
    }

    /// Copies info from `other` and merges into `self`:
    /// - Play counts are summed up
    /// - Ratings are averaged, or whichever one is non-zero is used
    /// - Modification time remains unchanged
    pub const fn merge_with(&mut self, other: &UserSongInfo) {
        self.play_count += other.play_count;
        if self.rating == 0 {
            self.rating = other.rating;
        } else if other.rating > 0 {
            self.rating = (self.rating + other.rating) / 2;
        }
    }
}

// IDEA: Make all fields optional, and load or access them on-demand
// using dedicated loader functions (e.g. `song.info().artwork()`)
// This might be useful once downscaled thumbnails are implemented
// However, there is an issue with that, because if there is no
// artwork assigned, the `artwork()` function would try to load it
// every time it is accessed, even though it does not exist
// One possible solution might be a nested `Option`
/// Fields which do not need to be held in memory at all times
#[derive(Clone)]
pub struct DetailedSongInfo {
    pub lyrics: String,
    pub artwork: Option<gdk::Texture>,
}

#[derive(Clone)]
pub enum LoadState<T: Clone> {
    Loaded(T),
    NotLoaded,
    Loading,
}

impl<'s> Song {
    /// Constructs a new `Song` from a `gio::File`
    #[inline]
    #[must_use]
    const fn from_file(file: gio::File) -> Song {
        Song {
            album: None,
            file,
            info: None,
            user_info: UserSongInfo::default(),
            detailed_info: LoadState::NotLoaded,
        }
    }
    /// Constructs a new `Song` from a file path
    #[inline]
    #[must_use]
    fn from_path(file: &str) -> Song {
        Song {
            album: None,
            file: gio::File::for_path(file),
            info: None,
            user_info: UserSongInfo::default(),
            detailed_info: LoadState::NotLoaded,
        }
    }

    /// Returns a `String` containing serialized `SongInfo` data,
    /// which can be used with the `deserialize()` method
    #[inline]
    #[must_use]
    pub fn serlialize(&mut self) -> String {
        let mut info = self.info();
        let uri = info.file_uri();
        let user_info = info.user().clone();
        let info = info.basic();

        serialize! {
            uri => "uri",
            info.title => "title",
            info.album => "album",
            info.artist => "artist",
            info.album_artist => "album_artist",
            info.track => "track",
            info.disc => "disc",
            info.year => "year",
            info.duration.nseconds() => "duration",
            user_info.modified => "modified",
            user_info.play_count => "play_count",
            user_info.rating => "rating",
        }
    }

    /// Loads the `data` and constructs a `Song` instance
    /// with parsed `SongInfo` values
    ///
    /// # Errors
    /// If a value cannot be parsed into the required type,
    /// the function returns an error
    #[inline]
    pub fn deserialize(data: &str) -> Result<Song, String> {
        let mut uri = "";
        let mut info = SongInfo::default();
        let mut user_info = UserSongInfo::default();

        deserialize! {
            data => {
                "uri"<"&str"> => uri,
                "title"<"String"> => info.title,
                "album"<"String"> => info.album,
                "artist"<"String"> => info.artist,
                "album_artist"<"String"> => info.album_artist,
                "track"<"parse"> => info.track,
                "disc"<"parse"> => info.disc,
                "year"<"parse"> => info.year,
                "duration"<"ClockTime"> => info.duration,
                "modified"<"parse"> => user_info.modified,
                "play_count"<"parse"> => user_info.play_count,
                "rating"<"parse"> => user_info.rating,
            }
        }

        if uri.is_empty() {
            return Err("Could not initialize `uri`".to_string());
        }

        Ok(Song {
            album: None,
            file: gio::File::for_uri(uri),
            info: Some(info),
            user_info,
            detailed_info: LoadState::NotLoaded,
        })
    }

    /// Returns a `SongInfoLoader`, which can be used to access information
    /// about the file and song. Tags are loaded on-demand, and remain in
    /// memory until the respective `unload` or `take` method is called.
    #[inline]
    #[must_use]
    pub fn info(&'s mut self) -> SongInfoLoader<'s> {
        #[cfg(debug_assertions)]
        if self.detailed_info.is_loading() {
            eprintln!(
                "WARNING: Mutex lock obtained while loading\n{}",
                Backtrace::capture()
            );
        }
        SongInfoLoader {
            file: &self.file,
            info: &mut self.info,
            user_info: &mut self.user_info,
            detailed_info: &mut self.detailed_info,
            tagged: None,
        }
    }
}

pub struct SongInfoLoader<'i> {
    file: &'i gio::File,
    info: &'i mut Option<SongInfo>,
    user_info: &'i mut UserSongInfo,
    detailed_info: &'i mut LoadState<DetailedSongInfo>,
    tagged: Option<TaggedFile>,
}

impl SongInfoLoader<'_> {
    /// Returns a reference to the `gio::File`
    #[must_use]
    pub const fn file(&self) -> &gio::File {
        self.file
    }

    /// Retruns the song file URI, which can be used by `GStreamer`
    #[inline]
    #[must_use]
    pub fn file_uri(&self) -> String {
        self.file.uri().to_string()
    }
    /// Returns the full song file path
    #[inline]
    #[must_use]
    pub fn file_path(&self) -> String {
        self.file.path().unwrap().to_str().unwrap().to_string()
    }
    /// Returns the song filename, including the file extestion
    #[inline]
    #[must_use]
    pub fn filename(&self) -> String {
        self.file.basename().map_or_else(
            || "Unknown".to_string(),
            |f| f.to_str().unwrap().to_string(),
        )
    }
    /// Returns the song file modification time
    #[must_use]
    pub fn file_modification_time(&self) -> i64 {
        self.file()
            .query_info(
                gio::FILE_ATTRIBUTE_TIME_MODIFIED,
                gio::FileQueryInfoFlags::empty(),
                gio::Cancellable::NONE,
            )
            .unwrap()
            .modification_date_time()
            .unwrap()
            .to_unix()
    }
    /// Last known modification time (Unix format); compare with
    /// `file_modification_time()` to detect modifications
    #[must_use]
    pub const fn known_modification_time(&self) -> i64 {
        self.user_info.modified
    }
    /// Updates the modification time to the current one from the file
    pub fn update_modification_time(&mut self) {
        self.user_info.modified = self.file_modification_time();
    }

    #[must_use]
    pub const fn user(&self) -> &UserSongInfo {
        self.user_info
    }

    #[must_use]
    pub const fn user_mut(&mut self) -> &mut UserSongInfo {
        self.user_info
    }

    /// Increases the play count by 1
    pub const fn played(&mut self) {
        self.user_info.play_count += 1;
    }

    /// Decreases the play count by 1
    pub const fn deduct_played(&mut self) {
        self.user_info.play_count -= 1;
    }

    /// Sets the song rating
    pub const fn set_rating(&mut self, rating: u8) {
        self.user_info.rating = rating;
    }

    /// Loads basic song info if needed, then returns it
    #[inline]
    #[must_use]
    pub fn basic(&mut self) -> &SongInfo {
        self.load_basic();
        // SAFETY: `load_basic()` ensures the value is `Some`
        unsafe { self.info.as_ref().unwrap_unchecked() }
    }
    /// Loads basic song info if needed, and runs the provided closure
    /// if the info had to be loaded
    /// Returns a reference to the loaded info
    #[inline]
    #[must_use]
    pub fn basic_and<F: FnOnce()>(&mut self, run_if_not_loaded: F) -> &SongInfo {
        if self.info.is_none() {
            self.load_basic();
            run_if_not_loaded();
        }
        // SAFETY: `load_basic()` ensures the value is `Some`
        unsafe { self.info.as_ref().unwrap_unchecked() }
    }
    /// Loads basic song info if needed, then returns and unloads it
    #[inline]
    #[must_use]
    pub fn take_basic(&mut self) -> SongInfo {
        self.load_basic();
        // SAFETY: `load_basic()` ensures the value is `Some`
        unsafe { self.info.take().unwrap_unchecked() }
    }
    /// Returns the basic song info if loaded, but does not load it
    #[inline]
    #[must_use]
    pub const fn inspect_basic(&self) -> Option<&SongInfo> {
        self.info.as_ref()
    }
    /// Loads basic song info so it is ready to be used later
    /// This method call can be chained
    #[inline]
    pub fn load_basic(&mut self) {
        if self.info.is_some() {
            return;
        }
        *self.info = self
            .load_basic_from_file()
            .inspect_err(|e| {
                eprintln!(
                    "Problem loading tags (basic): {:?}: {e}",
                    self.file.path().unwrap_or_default()
                );
            })
            .unwrap_or_else(|_| {
                Some(SongInfo {
                    title: self.filename(),
                    ..SongInfo::default()
                })
            });
    }
    /// Unloads basic song info
    #[inline]
    pub fn unload_basic(&mut self) {
        *self.info = None;
    }
    #[inline]
    fn load_basic_from_file(&mut self) -> Result<Option<SongInfo>, Box<dyn Error>> {
        if self.tagged.is_none() {
            self.tagged = Some(Probe::open(self.file.path().unwrap())?.read()?);
        }
        self.update_modification_time();
        let tagged = self.tagged.as_ref().unwrap();
        let tag = tagged
            .primary_tag()
            .or_else(|| tagged.first_tag())
            .ok_or("No tags found")?;
        let properties = tagged.properties();

        Ok(Some(SongInfo {
            title: tag.title().map_or_else(
                || self.filename(),
                |title| match title.trim().is_empty() {
                    true => self.filename(),
                    false => title.to_string(),
                },
            ),
            album: tag.album().unwrap_or_default().to_string(),
            artist: tag.artist().unwrap_or_default().to_string(),
            album_artist: tag.get_string(ItemKey::AlbumArtist).map_or_else(
                || tag.artist().unwrap_or_default().to_string(),
                |album_artist| album_artist.to_string(),
            ),
            track: tag.track().unwrap_or_default(),
            disc: tag.disk().unwrap_or(1),
            year: tag.date().unwrap_or_default().year,
            #[allow(clippy::cast_possible_truncation)]
            duration: ClockTime::from_mseconds(properties.duration().as_millis() as u64),
        }))
    }

    // TODO: Remove the `detailed` load methods in favor of `SharedSongExt` ones

    /// Loads detailed song info if needed, then returns it
    #[inline]
    #[must_use]
    pub fn detailed(&mut self) -> &DetailedSongInfo {
        self.load_detailed();
        // SAFETY: Relies on `load_detailed()` above to ensure the value is `Loaded`,
        unsafe { self.detailed_info.as_ref().unwrap_unchecked() }
    }
    /// Loads detailed song info if needed, and runs the provided closure
    /// if the info had to be loaded
    /// Returns a reference to the loaded info
    #[inline]
    #[must_use]
    pub fn detailed_and<F: FnOnce()>(&mut self, run_if_not_loaded: F) -> &DetailedSongInfo {
        if !self.detailed_info.is_loaded() {
            self.load_detailed();
            run_if_not_loaded();
        }
        // SAFETY: Relies on `load_detailed()` above to ensure the value is `Loaded`,
        unsafe { self.detailed_info.as_ref().unwrap_unchecked() }
    }
    /// Loads detailed song info if needed, then returns and unloads it
    #[inline]
    #[must_use]
    pub fn take_detailed(&mut self) -> DetailedSongInfo {
        self.load_detailed();
        // SAFETY: Relies on `load_detailed()` above to ensure the value is `Loaded`,
        unsafe { self.detailed_info.take().unwrap_unchecked() }
    }
    /// Returns the detailed song info if loaded, but does not load it
    #[inline]
    #[must_use]
    pub const fn inspect_detailed(&self) -> LoadState<&DetailedSongInfo> {
        self.detailed_info.as_ref()
    }
    /// Loads detailed song info so it is ready to be used later
    #[inline]
    pub fn load_detailed(&mut self) {
        // SAFETY: Ignoring the `Loading` state and just loading over it is intended,
        // because some other functions rely on `load_detailed` to always assign a
        // `Loaded` value to `self.detailed_info` which can be safely unwrapped with
        // `unwrap_unchecked`. Changing this could result in undefined behavior.
        if self.detailed_info.is_loaded() {
            return;
        }
        #[cfg(debug_assertions)]
        if self.detailed_info.is_loading() {
            eprintln!(
                "WARNING: Loading while another loading operation is ongoing. Will proceed because the other operation cannot complete due to the mutex lock.\n{}",
                Backtrace::capture()
            );
        }
        *self.detailed_info = match self
            .tagged_file()
            .map(|tagged| Self::load_tags_detailed(tagged))
        {
            Ok(Ok(result)) => result.map_or(
                LoadState::Loaded(DetailedSongInfo {
                    lyrics: String::new(),
                    artwork: None,
                }),
                LoadState::Loaded,
            ),
            Err(e) | Ok(Err(e)) => {
                eprintln!(
                    "Problem loading tags (detailed): {:?}: {e}",
                    self.file.path().unwrap_or_default()
                );
                LoadState::Loaded(DetailedSongInfo {
                    lyrics: String::new(),
                    artwork: None,
                })
            }
        };
    }
    /// Unloads detailed song info
    #[inline]
    pub fn unload_detailed(&mut self) {
        #[cfg(debug_assertions)]
        if self.detailed_info.is_loading() {
            eprintln!(
                "WARNING: Unloading while a loading operation is ongoing will result in the info being reassigned afterwards.\n{}",
                Backtrace::capture()
            );
        }
        *self.detailed_info = LoadState::NotLoaded;
    }

    /// Returns a new `TaggedFile` for reading song tags
    #[inline]
    fn tagged_file(&mut self) -> Result<&TaggedFile, Box<dyn Error>> {
        if self.tagged.is_none() {
            self.tagged = Some(Probe::open(self.file.path().unwrap())?.read()?);
        }
        Ok(self.tagged.as_ref().unwrap())
    }

    #[inline]
    fn load_tags_detailed(tagged: &TaggedFile) -> Result<Option<DetailedSongInfo>, Box<dyn Error>> {
        let tag = tagged
            .primary_tag()
            .or_else(|| tagged.first_tag())
            .ok_or("No tags found")?;
        Ok(Some(DetailedSongInfo {
            lyrics: tag
                .get_string(ItemKey::Lyrics)
                .unwrap_or_default()
                .to_string(),
            // TODO: Look for a `cover` file in the song directroy
            artwork: if tag.picture_count() > 0 {
                Some(gdk::Texture::from_bytes(&glib::Bytes::from(
                    tag.pictures()[0].data(),
                ))?)
            } else {
                None
            },
        }))
    }
}

impl<T: Clone> LoadState<T> {
    // Most of these were pretty much copied from Rust's `Option` source code

    // TODO: Add documentation

    #[inline]
    pub const fn is_loaded(&self) -> bool {
        matches!(self, LoadState::Loaded(_))
    }
    #[inline]
    pub const fn is_loading(&self) -> bool {
        matches!(self, LoadState::Loading)
    }
    #[inline]
    pub const fn can_load(&self) -> bool {
        matches!(self, LoadState::NotLoaded)
    }
    #[inline]
    pub fn is_some_and<F: FnOnce(T) -> bool>(self, f: F) -> bool {
        match self {
            LoadState::Loaded(x) => f(x),
            _ => false,
        }
    }
    #[inline]
    pub const fn as_ref(&self) -> LoadState<&T> {
        match self {
            LoadState::Loaded(x) => LoadState::Loaded(x),
            LoadState::Loading => LoadState::Loading,
            LoadState::NotLoaded => LoadState::NotLoaded,
        }
    }
    #[inline]
    pub fn map<R: Clone, F: FnOnce(&T) -> R>(&self, f: F) -> LoadState<R> {
        match self {
            LoadState::Loaded(x) => LoadState::Loaded(f(x)),
            LoadState::Loading => LoadState::Loading,
            LoadState::NotLoaded => LoadState::NotLoaded,
        }
    }
    pub fn map_or_else<U, D, F>(self, default: D, f: F) -> U
    where
        D: FnOnce() -> U,
        F: FnOnce(T) -> U,
    {
        match self {
            LoadState::Loaded(t) => f(t),
            _ => default(),
        }
    }
    /// Assumes the value is `Loaded` and unwraps it
    ///
    /// # Panics
    /// The function panics if the value is `Loading` or `NotLoaded`
    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            LoadState::Loaded(x) => x,
            _ => panic!("called `LoadState::unwrap()` on a value which was not `Ready`"),
        }
    }
    /// Assumes the value is `Loaded` and unwraps it without checking
    ///
    /// # Safety
    /// Caller must ensure to **never** call this on a `Loading`
    /// or `NotLoaded` value, as doing so is undefined behavior
    #[inline]
    pub unsafe fn unwrap_unchecked(self) -> T {
        match self {
            LoadState::Loaded(x) => x,
            // SAFETY: the safety contract must be upheld by the caller.
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
    /// Takes ownership of and returns the owned original value,
    /// leaving the field in `NotLoaded` state
    #[inline]
    pub const fn take(&mut self) -> LoadState<T> {
        mem::replace(self, LoadState::NotLoaded)
    }
}
