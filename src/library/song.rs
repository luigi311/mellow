use core::error::Error;
use gio::prelude::*;
use gst::ClockTime;
use gtk::{gdk, gio, glib};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard};

use lofty::file::TaggedFile;
use lofty::prelude::*;
use lofty::probe::Probe;

use crate::library::album::SharedAlbum;
use crate::{deserialize, serialize};

pub struct Song {
    album: Mutex<Option<SharedAlbum>>,
    file: gio::File,
    info: RwLock<Option<SongInfo>>,
    user_info: Mutex<UserSongInfo>,
    detailed_info: RwLock<Option<DetailedSongInfo>>,
}

pub type SharedSong = Arc<Song>;
pub trait SharedSongExt {
    fn from_file(file: gio::File) -> SharedSong;
    fn from_path(path: &str) -> SharedSong;
    fn deserialize(data: &str) -> Option<SharedSong>;
    fn album(&self) -> MutexGuard<'_, Option<SharedAlbum>>;
    fn set_album(&self, album: SharedAlbum);
}
impl SharedSongExt for SharedSong {
    /// Constructs a new `SharedSong` from a `gio::File`
    #[inline]
    fn from_file(file: gio::File) -> SharedSong {
        Arc::new(Song::from_file(file))
    }
    /// Constructs a new `SharedSong` from a file path
    #[inline]
    fn from_path(path: &str) -> SharedSong {
        Arc::new(Song::from_path(path))
    }
    /// Constructs a new `SharedSong` using serialized data
    /// Returns `Some` on successful load, or `None`
    #[inline]
    fn deserialize(data: &str) -> Option<SharedSong> {
        Song::deserialize(data).map_or_else(|_| None, |song| Some(Arc::new(song)))
    }
    /// Returns the currently assigned album's `MutexGuard`
    #[inline]
    fn album(&self) -> MutexGuard<'_, Option<SharedAlbum>> {
        self.album.lock().unwrap()
    }
    // Sets `self.album` to the given `album`
    #[inline]
    fn set_album(&self, album: SharedAlbum) {
        *self.album.lock().unwrap() = Some(album);
    }
}

impl<'s> Song {
    /// Constructs a new `Song` from a `gio::File`
    #[inline]
    #[must_use]
    const fn from_file(file: gio::File) -> Song {
        Song {
            album: Mutex::new(None),
            file,
            info: RwLock::new(None),
            user_info: Mutex::new(UserSongInfo::default()),
            detailed_info: RwLock::new(None),
        }
    }
    /// Constructs a new `Song` from a file path
    #[inline]
    #[must_use]
    fn from_path(file: &str) -> Song {
        Song {
            album: Mutex::new(None),
            file: gio::File::for_path(file),
            info: RwLock::new(None),
            user_info: Mutex::new(UserSongInfo::default()),
            detailed_info: RwLock::new(None),
        }
    }

    /// Returns a `String` containing serialized `SongInfo` data,
    /// which can be used with the `deserialize()` method
    #[inline]
    #[must_use]
    pub fn serlialize(&self) -> String {
        let mut info = self.info();
        let uri = info.file_uri();
        let user_info = info.user().clone();
        let info = info.load_basic();
        // SAFETY: `info.load_basic` is always safe to unwrap
        let info = unsafe { info.as_ref().unwrap_unchecked() };

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
    fn deserialize(data: &str) -> Result<Song, String> {
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
            album: Mutex::new(None),
            file: gio::File::for_uri(uri),
            info: RwLock::new(Some(info)),
            user_info: Mutex::new(user_info),
            detailed_info: RwLock::new(None),
        })
    }

    /// Returns a `SongInfoLoader`, which can be used to access information
    /// about the file and song. Tags are loaded on-demand, and remain in
    /// memory until the respective `unload` or `take` method is called.
    #[inline]
    #[must_use]
    pub const fn info(&'s self) -> SongInfoLoader<'s> {
        SongInfoLoader {
            file: &self.file,
            info: &self.info,
            user_info: &self.user_info,
            detailed_info: &self.detailed_info,
            tagged: None,
        }
    }
}

pub struct TryLockError;

pub struct SongInfoLoader<'i> {
    file: &'i gio::File,
    info: &'i RwLock<Option<SongInfo>>,
    user_info: &'i Mutex<UserSongInfo>,
    detailed_info: &'i RwLock<Option<DetailedSongInfo>>,
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
    ///
    /// # Panics
    /// The function panics if the filename is not valid UTF-8
    #[inline]
    #[must_use]
    pub fn filename(&self) -> String {
        self.file.basename().map_or_else(
            || "Unknown".to_string(),
            |f| f.to_str().unwrap().to_string(),
        )
    }
    /// Returns the song file modification time
    #[inline]
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
    ///
    /// # Panics
    /// The function panics if the user info `Mutex` is poisoned
    #[must_use]
    pub fn known_modification_time(&self) -> i64 {
        self.user_info.lock().unwrap().modified
    }
    /// Updates the modification time to the current one from the file
    ///
    /// # Panics
    /// The function panics if the user info `Mutex` is poisoned
    pub fn update_modification_time(&self) {
        self.user_info.lock().unwrap().modified = self.file_modification_time();
    }

    /// Returns the user song info `MutexGuard`
    ///
    /// # Panics
    /// The function panics if the user info `Mutex` is poisoned
    pub fn user(&self) -> MutexGuard<'_, UserSongInfo> {
        self.user_info.lock().unwrap()
    }

    /// Increases the play count by 1
    ///
    /// # Panics
    /// The function panics if the user info `Mutex` is poisoned
    pub fn played(&self) {
        self.user_info.lock().unwrap().play_count += 1;
    }

    /// Decreases the play count by 1
    ///
    /// # Panics
    /// The function panics if the user info `Mutex` is poisoned
    pub fn deduct_played(&self) {
        self.user_info.lock().unwrap().play_count -= 1;
    }

    /// Sets the song rating
    ///
    /// # Panics
    /// The function panics if the user info `RwLock` is poisoned
    pub fn set_rating(&self, rating: u8) {
        self.user_info.lock().unwrap().rating = rating;
    }

    /// Loads basic song info if needed, then returns and unloads it
    ///
    /// # Panics
    /// The function panics if the basic info `RwLock` is poisoned
    #[inline]
    #[must_use]
    pub fn take_basic(&mut self) -> SongInfo {
        drop(self.load_basic());
        // SAFETY: `load_basic()` ensures the value is `Some`
        unsafe { self.info.write().unwrap().take().unwrap_unchecked() }
    }
    /// Returns the basic song info if loaded, but does not load it
    ///
    /// # Panics
    /// The function panics if the basic info `RwLock` is poisoned
    #[inline]
    pub fn inspect_basic(&self) -> RwLockReadGuard<'_, Option<SongInfo>> {
        self.info.read().unwrap()
    }
    /// Loads basic song info and returns its `MutexGuard`.
    /// The returned inner `Option` is always safe to unwrap.
    ///
    /// # Panics
    /// The function panics if the basic info `RwLock` is poisoned
    #[inline]
    pub fn load_basic(&mut self) -> RwLockReadGuard<'_, Option<SongInfo>> {
        let info = self.info.read().unwrap();
        if info.is_some() {
            return info;
        }
        drop(info);
        let mut info_writer = self.info.write().unwrap();
        // Check if the info was already loaded by another
        // writer while waiting to acquire the write lock
        #[cfg(debug_assertions)]
        if info_writer.is_some() {
            println!(
                "⚠️ Basic song info already loaded - enable the check for release builds as well"
            );
            drop(info_writer);
            return self.info.read().unwrap();
        }
        info_writer.replace(self.basic_or_default());
        drop(info_writer);
        self.info.read().unwrap()
    }
    /// Returns the basic song info if it is currently accessible without
    /// blocking the thread.
    /// The returned inner `Option` of the `Ok` variant is always safe to unwrap.
    ///
    /// # Errors
    /// - If the info is loaded, but cannot be read without blocking
    ///
    /// # Panics
    /// The function panics if the basic info `RwLock` is poisoned
    #[inline]
    pub fn try_load_basic(
        &mut self,
    ) -> Result<RwLockReadGuard<'_, Option<SongInfo>>, TryLockError> {
        let Ok(info) = self.info.try_read() else {
            return Err(TryLockError);
        };
        if info.is_some() {
            return Ok(info);
        }
        drop(info);
        let mut info_writer = self.info.write().unwrap();
        // Check if the info was already loaded by another
        // writer while waiting to acquire the write lock
        #[cfg(debug_assertions)]
        if info_writer.is_some() {
            println!(
                "⚠️ Basic song info already loaded - enable the check for release builds as well"
            );
            drop(info_writer);
            return Ok(self.info.read().unwrap());
        }
        info_writer.replace(self.basic_or_default());
        drop(info_writer);
        Ok(self.info.read().unwrap())
    }
    #[inline]
    fn basic_or_default(&mut self) -> SongInfo {
        self.load_basic_from_file().unwrap_or_else(|e| {
            eprintln!(
                "Problem loading tags (basic): {:?}: {e}",
                self.file.path().unwrap_or_default()
            );
            SongInfo {
                title: self.filename(),
                ..SongInfo::default()
            }
        })
    }
    /// Unloads basic song info
    ///
    /// # Panics
    /// The function panics if the basic info `RwLock` is poisoned
    #[inline]
    pub fn unload_basic(&mut self) {
        *self.info.write().unwrap() = None;
    }
    #[inline]
    fn load_basic_from_file(&mut self) -> Result<SongInfo, Box<dyn Error>> {
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

        Ok(SongInfo {
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
        })
    }

    /// Loads detailed song info if needed, then returns and unloads it
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    #[must_use]
    pub fn take_detailed(&mut self) -> DetailedSongInfo {
        drop(self.load_detailed());
        // SAFETY: `load_detailed()` ensures the value is `Some`
        unsafe {
            self.detailed_info
                .write()
                .unwrap()
                .take()
                .unwrap_unchecked()
        }
    }
    /// Returns the detailed song info if loaded, but does not load it
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    pub fn inspect_detailed(&self) -> RwLockReadGuard<'_, Option<DetailedSongInfo>> {
        self.detailed_info.read().unwrap()
    }
    /// Loads detailed song info and returns its `MutexGuard`.
    /// The returned inner `Option` is always safe to unwrap.
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    pub fn load_detailed(&mut self) -> RwLockReadGuard<'_, Option<DetailedSongInfo>> {
        let detailed_info = self.detailed_info.read().unwrap();
        if detailed_info.is_some() {
            return detailed_info;
        }
        drop(detailed_info);
        let mut info_writer = self.detailed_info.write().unwrap();
        // Check if the info was already loaded by another
        // writer while waiting to acquire the write lock
        #[cfg(debug_assertions)]
        if info_writer.is_some() {
            println!(
                "⚠️ Detailed song info already loaded - enable the check for release builds as well"
            );
            drop(info_writer);
            return self.detailed_info.read().unwrap();
        }
        info_writer.replace(self.detailed_or_default());
        drop(info_writer);
        self.detailed_info.read().unwrap()
    }
    /// Returns the detailed song info if it is currently accessible without
    /// blocking the thread.
    /// The returned inner `Option` of the `Ok` variant is always safe to unwrap.
    ///
    /// # Errors
    /// - If the info is loaded, but cannot be read without blocking
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    pub fn try_load_detailed(
        &mut self,
    ) -> Result<RwLockReadGuard<'_, Option<DetailedSongInfo>>, TryLockError> {
        let Ok(detailed_info) = self.detailed_info.try_read() else {
            return Err(TryLockError);
        };
        if detailed_info.is_some() {
            return Ok(detailed_info);
        }
        let mut info_writer = self.detailed_info.write().unwrap();
        // Check if the info was already loaded by another
        // writer while waiting to acquire the write lock
        #[cfg(debug_assertions)]
        if info_writer.is_some() {
            println!(
                "⚠️ Detailed song info already loaded - enable the check for release builds as well"
            );
            drop(info_writer);
            return Ok(self.detailed_info.read().unwrap());
        }
        info_writer.replace(self.detailed_or_default());
        drop(info_writer);
        Ok(self.detailed_info.read().unwrap())
    }
    /// Attempts to read detailed info from tags and returns it,
    /// or returns a default value if it cannot
    #[inline]
    fn detailed_or_default(&mut self) -> DetailedSongInfo {
        match self
            .tagged_file()
            .map(|tagged| Self::load_tags_detailed(tagged))
        {
            Ok(Ok(result)) => result,
            Err(e) | Ok(Err(e)) => {
                eprintln!(
                    "Problem loading tags (detailed): {:?}: {e}",
                    self.file.path().unwrap_or_default()
                );
                DetailedSongInfo {
                    lyrics: String::new(),
                    artwork: None,
                }
            }
        }
    }
    /// Unloads detailed song info
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    pub fn unload_detailed(&self) {
        *self.detailed_info.write().unwrap() = None;
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
    fn load_tags_detailed(tagged: &TaggedFile) -> Result<DetailedSongInfo, Box<dyn Error>> {
        let tag = tagged
            .primary_tag()
            .or_else(|| tagged.first_tag())
            .ok_or("No tags found")?;
        Ok(DetailedSongInfo {
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
        })
    }
}

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
#[derive(Clone, Debug)]
pub struct UserSongInfo {
    pub modified: i64,
    pub play_count: usize,
    pub rating: u8,
}
/// Fields which do not need to be held in memory at all times
pub struct DetailedSongInfo {
    pub lyrics: String,
    pub artwork: Option<gdk::Texture>,
}

impl PartialEq for SongInfo {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title
            && self.album == other.album
            && self.artist == other.artist
            && self.track == other.track
    }
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
