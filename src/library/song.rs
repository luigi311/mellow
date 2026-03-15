use core::error::Error;
use gdk::{gdk_pixbuf::Pixbuf, prelude::*};
use gtk::{gdk, gio, glib};
use std::fs::{self, File};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Read;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use lofty::file::TaggedFile;
use lofty::prelude::*;
use lofty::probe::Probe;

use crate::excuses::EXP_INIT;
use crate::library::SharedAlbum;
use crate::util::{deserialize, serialize, serialize_list};
use crate::{CACHE_DIR, util::unescaped_split};

pub struct Song {
    album: OnceLock<SharedAlbum>,
    file: gio::File,
    info: RwLock<Option<SongInfo>>,
    user_info: Mutex<UserSongInfo>,
    detailed_info: RwLock<Option<DetailedSongInfo>>,
    thumbnail: RwLock<Option<gdk::Texture>>,
}

pub type SharedSong = Arc<Song>;
pub trait SharedSongExt {
    fn from_file(file: gio::File) -> SharedSong;
    fn from_path(path: &str) -> SharedSong;
    fn deserialize(data: &str) -> Option<SharedSong>;
    fn album(&self) -> Option<&SharedAlbum>;
    fn get_album(&self) -> SharedAlbum;
    fn set_album(&self, album: SharedAlbum) -> Result<(), SharedAlbum>;
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
    /// Returns a reference the currently assigned album if assigned
    #[inline]
    fn album(&self) -> Option<&SharedAlbum> {
        self.album.get()
    }
    /// Returns a cloned reference to the currently assigned `SharedAlbum`
    ///
    /// # Panics
    /// Panics if the `album` has not been initialized
    #[inline]
    fn get_album(&self) -> SharedAlbum {
        Arc::clone(self.album.get().expect(EXP_INIT))
    }
    /// Sets `self.album` to the given `album`
    ///
    /// # Errors
    /// Returns an error containing the assigned value
    /// if the `album` has already been already set
    #[inline]
    fn set_album(&self, album: SharedAlbum) -> Result<(), SharedAlbum> {
        self.album.set(album)
    }
}

impl<'s> Song {
    /// Constructs a new `Song` from a `gio::File`
    #[inline]
    #[must_use]
    fn from_file(file: gio::File) -> Song {
        Song {
            album: OnceLock::new(),
            file,
            info: RwLock::new(None),
            user_info: Mutex::new(UserSongInfo::default()),
            detailed_info: RwLock::new(None),
            thumbnail: RwLock::new(None),
        }
    }
    /// Constructs a new `Song` from a file path
    #[inline]
    #[must_use]
    fn from_path(file: &str) -> Song {
        Song {
            album: OnceLock::new(),
            file: gio::File::for_path(file),
            info: RwLock::new(None),
            user_info: Mutex::new(UserSongInfo::default()),
            detailed_info: RwLock::new(None),
            thumbnail: RwLock::new(None),
        }
    }

    /// Returns a `String` containing serialized song info,
    /// which can be used with the `deserialize()` method
    /// If the song info is not loaded, only the user info
    /// is serialized
    #[inline]
    #[must_use]
    pub fn serlialize(&self) -> String {
        let info = self.info();
        let uri = info.file_uri();
        let user_info = info.user().clone();
        (info.inspect_basic().as_ref()).map_or_else(
            || {
                serialize! {
                    uri => "uri",
                    user_info.added => "added",
                    0 => "modified",
                    user_info.play_count => "play_count",
                    user_info.rating => "rating",
                    serialize_list(&user_info.tags) => "tags",
                }
            },
            |info| {
                serialize! {
                    uri => "uri",
                    user_info.added => "added",
                    user_info.modified => "modified",
                    info.title => "title",
                    info.album => "album",
                    info.artist => "artist",
                    info.album_artist => "album_artist",
                    info.track => "track",
                    info.disc => "disc",
                    info.year => "year",
                    info.duration_ms => "duration",
                    user_info.play_count => "play_count",
                    user_info.rating => "rating",
                    serialize_list(&user_info.tags) => "tags",
                }
            },
        )
    }

    /// Returns an `Option<String>` containing serialized song info,
    /// which can be used with the `deserialize()` method.
    /// If the song info is not loaded, the function returns `None`.
    #[inline]
    #[must_use]
    pub fn try_serlialize(&self) -> Option<String> {
        let info = self.info();
        let uri = info.file_uri();
        let user_info = info.user().clone();
        (info.inspect_basic().as_ref()).map(|info| {
            serialize! {
                uri => "uri",
                user_info.added => "added",
                user_info.modified => "modified",
                info.title => "title",
                info.album => "album",
                info.artist => "artist",
                info.album_artist => "album_artist",
                info.track => "track",
                info.disc => "disc",
                info.year => "year",
                info.duration_ms => "duration",
                user_info.play_count => "play_count",
                user_info.rating => "rating",
                serialize_list(&user_info.tags) => "tags",
            }
        })
    }

    /// Constructs a new `Song` instance using the serialized song info `data`
    ///
    /// # Panics
    /// - If a value cannot be parsed into the required type
    ///
    /// # Errors
    /// - If the `uri` field is missing from the `data`
    #[inline]
    fn deserialize(data: &str) -> Result<Song, String> {
        let mut uri = "";
        let mut info = SongInfo::default();
        let mut user_info = UserSongInfo::default();

        deserialize! {
            data => {
                "uri"<str> => uri,
                "added"<?> => user_info.added,
                "modified"<?> => user_info.modified,
                "title"<String> => info.title,
                "album"<String> => info.album,
                "artist"<String> => info.artist,
                "album_artist"<String> => info.album_artist,
                "track"<?> => info.track,
                "disc"<?> => info.disc,
                "year"<?> => info.year,
                "duration"<?> => info.duration_ms,
                "play_count"<?> => user_info.play_count,
                "rating"<?> => user_info.rating,
                "tags"<[String]> => user_info.tags,
            }
        }

        if uri.is_empty() {
            return Err("Could not initialize `uri`".to_owned());
        }

        Ok(Song {
            album: OnceLock::new(),
            file: gio::File::for_uri(uri),
            info: RwLock::new(match user_info.modified {
                0 => None,
                _ => Some(info),
            }),
            user_info: Mutex::new(user_info),
            detailed_info: RwLock::new(None),
            thumbnail: RwLock::new(None),
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
            thumbnail: &self.thumbnail,
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
    thumbnail: &'i RwLock<Option<gdk::Texture>>,
    tagged: Option<TaggedFile>,
}

impl SongInfoLoader<'_> {
    /// Whether the two `SongInfoLoader`s are likely to belong to the same `Song`
    ///
    /// Note: if either `SongInfo` is not loaded, equality is determined using the
    /// file URIs only. For more accurate matching, calling `load_basic` beforehand
    /// might be preferable.
    #[inline]
    #[must_use]
    pub fn matches(&self, other: &SongInfoLoader) -> bool {
        self.inspect_basic().as_ref().map_or_else(
            || self.file_uri() == other.file_uri(),
            |own_info| {
                other.inspect_basic().as_ref().map_or_else(
                    || self.file_uri() == other.file_uri(),
                    |other_info| own_info == other_info,
                )
            },
        )
    }

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
    /// Returns the hash of the `file_uri`, used for thumbnail files
    #[inline]
    #[must_use]
    pub fn uri_hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.file_uri().hash(&mut hasher);
        hasher.finish().to_string()
    }
    /// Returns this song's thumbnail file path
    #[inline]
    #[must_use]
    pub fn thumbnail_file_path(&self) -> String {
        [
            CACHE_DIR.get().expect(EXP_INIT),
            "thumbnails/",
            &self.uri_hash(),
        ]
        .concat()
    }
    /// Returns the full song file path
    ///
    /// # Panics
    /// The function panics if the path is not valid UTF-8
    #[inline]
    #[must_use]
    pub fn file_path(&self) -> String {
        self.file.path().unwrap().to_str().unwrap().to_owned()
    }
    /// Returns the song filename, including the file extestion
    ///
    /// # Panics
    /// The function panics if the filename is not valid UTF-8
    #[inline]
    #[must_use]
    pub fn filename(&self) -> String {
        self.file.basename().map_or_else(
            || String::from("Unknown"),
            |f| f.to_str().unwrap().to_owned(),
        )
    }
    /// Determines a fallback title using the filename
    #[inline]
    #[must_use]
    fn fallback_title(&self) -> String {
        (self.filename().rsplit_once('.')).map_or(String::new(), |name| name.0.to_owned())
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
    /// Sets the known modification time to the provided value
    ///
    /// # Panics
    /// The function panics if the user info `Mutex` is poisoned
    pub fn set_modification_time(&self, time: i64) {
        self.user_info.lock().unwrap().modified = time;
    }

    /// Returns the user song info `MutexGuard`
    ///
    /// # Panics
    /// The function panics if the user info `Mutex` is poisoned
    pub fn user(&self) -> MutexGuard<'_, UserSongInfo> {
        #[cfg(debug_assertions)]
        if self.user_info.try_lock().is_err() {
            eprintln!("Note: Blocking on read lock for `user`");
        }
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
    /// Note: This function may block the current thread if the song
    /// info is already being loaded elsewhere; if this is not desired,
    /// use `try_inspect_basic` instead
    ///
    /// # Panics
    /// The function panics if the basic info `RwLock` is poisoned
    #[inline]
    pub fn inspect_basic(&self) -> RwLockReadGuard<'_, Option<SongInfo>> {
        #[cfg(debug_assertions)]
        if self.detailed_info.try_read().is_err() {
            eprintln!(
                "Note: Blocking on read lock for `inspect_basic` (would `try_inspect_basic` make sense here?)"
            );
        }
        self.info.read().unwrap()
    }
    /// Returns the basic song info if loaded, but does not load it
    ///
    /// This function blocks until the `RwLock` write lock can be obtained
    ///
    /// # Panics
    /// The function panics if the basic info `RwLock` is poisoned
    #[inline]
    pub fn inspect_basic_mut(&mut self) -> RwLockWriteGuard<'_, Option<SongInfo>> {
        #[cfg(debug_assertions)]
        if self.detailed_info.try_write().is_err() {
            eprintln!(
                "Note: Blocking on write lock for `inspect_basic` (would `try_inspect_basic` make sense here?)"
            );
        }
        self.info.write().unwrap()
    }
    /// Returns the basic song info if accessible without blocking
    /// the current thread, but does not load it
    ///
    /// # Errors
    /// The function errors if the `RwLock` is currently busy
    #[inline]
    pub fn try_inspect_basic(&self) -> Result<RwLockReadGuard<'_, Option<SongInfo>>, TryLockError> {
        self.info.try_read().map_err(|_| TryLockError)
    }
    /// Loads basic song info and returns its `MutexGuard`.
    /// The returned inner `Option` is always safe to unwrap.
    ///
    /// # Panics
    /// The function panics if the basic info `RwLock` is poisoned
    #[inline]
    pub fn load_basic(&mut self) -> RwLockReadGuard<'_, Option<SongInfo>> {
        #[cfg(debug_assertions)]
        if self.detailed_info.try_write().is_err() {
            eprintln!(
                "Note: Blocking on read lock for `load_basic` (would `try_load_basic` make sense here?)"
            );
        }
        let info = self.info.read().unwrap();
        if info.is_some() {
            return info;
        }
        drop(info);
        self.assign_basic();
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
        self.assign_basic();
        Ok(self.info.read().unwrap())
    }
    /// Loads the basic song info and assigns it if it is not already loaded
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    fn assign_basic(&mut self) {
        #[cfg(debug_assertions)]
        if self.detailed_info.try_write().is_err() {
            eprintln!("Note: Blocking on write lock for `assign_basic`");
        }
        let mut info_writer = self.info.write().unwrap();
        // Check if the info was already loaded by another
        // writer while waiting to acquire the write lock
        if info_writer.is_some() {
            return;
        }
        *info_writer = Some(self.basic_or_default());
    }
    /// Reads and returns the basic song info from file,
    /// or returns a fallback if unavailable
    #[inline]
    fn basic_or_default(&mut self) -> SongInfo {
        self.load_basic_from_file().unwrap_or_else(|e| {
            eprintln!(
                "Problem loading tags (basic): {:?}: {e}",
                self.file.path().unwrap_or_default()
            );
            SongInfo {
                title: self.fallback_title(),
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
                || self.fallback_title(),
                |title| match title.trim().is_empty() {
                    true => self.fallback_title(),
                    false => title.to_string(),
                },
            ),
            album: tag.album().unwrap_or_default().to_string(),
            artist: tag.artist().unwrap_or_default().to_string(),
            album_artist: tag.get_string(ItemKey::AlbumArtist).map_or_else(
                || tag.artist().unwrap_or_default().to_string(),
                |album_artist| album_artist.to_owned(),
            ),
            track: tag.track().unwrap_or_default(),
            disc: tag.disk().unwrap_or(1),
            year: tag.date().unwrap_or_default().year,
            #[allow(clippy::cast_possible_truncation)]
            duration_ms: properties.duration().as_millis() as u64,
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
        unsafe { (self.detailed_info.write().unwrap().take()).unwrap_unchecked() }
    }
    /// Returns the detailed song info if loaded, but does not load it
    ///
    /// Note: This function may block the current thread if the song
    /// info is already being loaded elsewhere; if this is not desired,
    /// use `try_inspect_detailed` instead
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    pub fn inspect_detailed(&self) -> RwLockReadGuard<'_, Option<DetailedSongInfo>> {
        #[cfg(debug_assertions)]
        if self.detailed_info.try_read().is_err() {
            eprintln!(
                "Note: Blocking on read lock for `inspect_detailed` (would `try_inspect_detailed` make sense here?)"
            );
        }
        self.detailed_info.read().unwrap()
    }
    /// Returns the basic song info if accessible without blocking
    /// the current thread, but does not load it
    ///
    /// # Errors
    /// The function errors if the `RwLock` is currently busy
    #[inline]
    pub fn try_inspect_detailed(
        &self,
    ) -> Result<RwLockReadGuard<'_, Option<DetailedSongInfo>>, TryLockError> {
        self.detailed_info.try_read().map_err(|_| TryLockError)
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
        self.assign_detailed();
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
        self.assign_detailed();
        Ok(self.detailed_info.read().unwrap())
    }
    /// Loads the detailed song info and assigns it if it is not already loaded
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    fn assign_detailed(&mut self) {
        let mut info_writer = self.detailed_info.write().unwrap();
        // Check if the info was already loaded by another
        // writer while waiting to acquire the write lock
        if info_writer.is_some() {
            #[cfg(debug_assertions)]
            println!(
                "⚠️ Detailed song info already loaded (decide whether to include this check it in release builds) ({})",
                line!()
            );
            return;
        }
        *info_writer = Some(self.detailed_or_default());
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
        #[cfg(debug_assertions)]
        if self.detailed_info.try_write().is_err() {
            eprintln!(
                "Note: Blocking on write lock for `unload_detailed` (would `try_unload_detailed` make sense here?)"
            );
        }
        *self.detailed_info.write().unwrap() = None;
    }
    /// Unloads detailed song info if the write lock can be
    /// obtained without blocking, or does nothing otherwise
    #[inline]
    pub fn try_unload_detailed(&self) {
        if let Ok(mut detailed_info) = self.detailed_info.try_write() {
            *detailed_info = None;
        }
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
        // TODO: Would it be possible to cancel artowrk loading while it is in progress?
        let tag = tagged
            .primary_tag()
            .or_else(|| tagged.first_tag())
            .ok_or("No tags found")?;
        Ok(DetailedSongInfo {
            lyrics: tag
                .get_string(ItemKey::Lyrics)
                .unwrap_or_default()
                .to_owned(),
            // TODO: Look for a `cover` file in the song directroy
            // IDEA: Once `cover` files are supported, load both and compare their resolutions
            // and average color delta (to see if they differ) to pick the best one
            // (for average colors, the logic could be factored out from the `settings_page`)
            artwork: if tag.picture_count() > 0 {
                Some(gdk::Texture::from_bytes(&glib::Bytes::from(
                    tag.pictures()[0].data(),
                ))?)
            } else {
                None
            },
        })
    }

    /// Loads the thumbnail or creates it if necessary
    ///
    /// Note: The returned inner `Option` could be `None`
    /// if the file does not have an artwork available
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    pub fn load_thumbnail(&mut self) -> RwLockReadGuard<'_, Option<gdk::Texture>> {
        #[cfg(debug_assertions)]
        if self.thumbnail.try_read().is_err() {
            println!(
                "Note: Blocking on read lock for `load_thumbnail` (would `try_load_thumbnail` make sense here?)"
            );
        }
        let thumbnail = self.thumbnail.read().unwrap();
        if thumbnail.is_some() {
            // println!("Thumbnail already loaded, nothing to do");
            return thumbnail;
        }
        drop(thumbnail);

        #[cfg(debug_assertions)]
        if self.thumbnail.try_write().is_err() {
            println!(
                "Note: Blocking on write lock for `load_thumbnail` (would `try_load_thumbnail` make sense here?)"
            );
        }
        if let Ok(thumbnail) = self.read_thumbnail_from_disk() {
            // println!("Thumbnail was read successfully from disk");
            *self.thumbnail.write().unwrap() = thumbnail;
        } else {
            // println!("Creating a new thumbnail");
            self.create_thumbnail();
        }

        self.thumbnail.read().unwrap()
    }
    /// Returns the thumbnail, but does not load it
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    pub fn inspect_thumbnail(&self) -> RwLockReadGuard<'_, Option<gdk::Texture>> {
        #[cfg(debug_assertions)]
        if self.thumbnail.try_read().is_err() {
            println!(
                "Note: Blocking on read lock for `inspect_thumbnail` (would `try_inspect_thumbnail` make sense here?)"
            );
        }
        self.thumbnail.read().unwrap()
    }
    /// Returns the thumbnail if accessible without blocking
    /// the current thread, but does not load it
    ///
    /// # Errors
    /// The function errors if the `RwLock` is currently busy
    #[inline]
    pub fn try_inspect_thumbnail(
        &self,
    ) -> Result<RwLockReadGuard<'_, Option<gdk::Texture>>, TryLockError> {
        self.thumbnail.try_read().map_err(|_| TryLockError)
    }
    /// Unloads the song's thumbnail from memory if it is no longer used
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    pub fn unload_thumbnail(&mut self) {
        let Ok(mut writer) = self.thumbnail.write() else {
            return;
        };
        if writer.as_ref().is_some_and(|t| t.ref_count() < 2) {
            *writer = None;
        }
    }
    /// Unloads the song's thumbnail from memory if it is no longer used,
    /// but only if possible to do so without blocking
    #[inline]
    pub fn try_unload_thumbnail(&mut self) {
        let Ok(mut writer) = self.thumbnail.try_write() else {
            return;
        };
        if writer.as_ref().is_some_and(|t| t.ref_count() < 2) {
            *writer = None;
        }
    }
    /// Unloads the song's thumbnail form memory and disk
    ///
    /// # Panics
    /// The function panics if the detailed info `RwLock` is poisoned
    #[inline]
    pub fn invalidate_thumbnail(&mut self) {
        let _ = fs::remove_file(self.thumbnail_file_path());
        self.unload_thumbnail();
    }
    #[inline]
    fn read_thumbnail_from_disk(&self) -> Result<Option<gdk::Texture>, Box<dyn Error>> {
        let mut thumbnail_file = File::open(self.thumbnail_file_path())?;
        let mut buffer = Vec::new();
        thumbnail_file.read_to_end(&mut buffer).unwrap();
        Ok(gdk::Texture::from_bytes(&glib::Bytes::from(&*buffer)).ok())
    }
    fn create_thumbnail(&mut self) {
        let thumbnail_file_path = self.thumbnail_file_path();
        fs::create_dir_all(thumbnail_file_path.rsplit_once('/').unwrap().0).unwrap();

        let detailed = self.load_detailed();
        // SAFETY: `load_detailed` ensures the value is `Some`
        let artwork = unsafe { detailed.as_ref().unwrap_unchecked() }
            .artwork
            .clone();
        drop(detailed);

        let thumbnail = 'thumbnail: {
            let Some(artwork) = artwork else {
                break 'thumbnail None;
            };
            let mut tex_dl = gdk::TextureDownloader::new(&artwork);
            tex_dl.set_format(gdk::MemoryFormat::R8g8b8a8Premultiplied);
            let (bytes, row_stride) = tex_dl.download_bytes();
            let pixbuf = Pixbuf::from_bytes(
                &bytes,
                gtk::gdk_pixbuf::Colorspace::Rgb,
                true,
                8,
                artwork.width(),
                artwork.height(),
                row_stride as i32,
            )
            .scale_simple(
                256,
                (256.0 / artwork.intrinsic_aspect_ratio()) as i32,
                gtk::gdk_pixbuf::InterpType::Bilinear,
            )
            .unwrap();

            // FIX: `gdk::Texture::for_pixbuf` is deprecated
            Some(gdk::Texture::for_pixbuf(&pixbuf))
            // gdk::Texture::from_bytes(&pixbuf.read_pixel_bytes())
            //     .inspect_err(|e| eprintln!("{e}"))
            //     .ok()
        };

        match &thumbnail {
            Some(thumbnail) => thumbnail.save_to_png(thumbnail_file_path).unwrap(),
            None => fs::write(thumbnail_file_path, "").unwrap(),
        }

        *self.thumbnail.write().unwrap() = thumbnail;
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
    pub duration_ms: u64,
}
#[derive(Clone, Debug)]
pub struct UserSongInfo {
    pub added: u64,
    pub modified: i64,
    pub play_count: usize,
    pub rating: u8,
    pub tags: Vec<String>,
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
            duration_ms: 0,
        }
    }
}

impl Default for UserSongInfo {
    fn default() -> Self {
        Self {
            added: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or_else(|_| 0, |time| time.as_secs()),
            modified: 0,
            play_count: 0,
            rating: 0,
            tags: Vec::new(),
        }
    }
}
impl UserSongInfo {
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
