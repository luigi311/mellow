use core::error::Error;
use gio::prelude::*;
use gst::ClockTime;
use gtk::{gdk, gio, glib};
use std::sync::{Arc, Mutex};

use lofty::file::TaggedFile;
use lofty::prelude::*;
use lofty::probe::Probe;

use crate::library::album::AlbumMutex;
use crate::{deserialize, serialize};

pub type SongMutex = Arc<Mutex<Song>>;

#[derive(Clone)]
pub struct Song {
    pub album: Option<AlbumMutex>,
    file: gio::File,
    info: Option<SongInfo>,
    user_info: UserSongInfo,
    detailed_info: Option<DetailedSongInfo>,
}

#[derive(Clone)]
pub struct SongInfo {
    pub title: String,
    pub album: String,
    pub artist: String,
    pub album_artist: String,
    pub track: u32,
    pub disc: u32,
    pub year: u32,
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

#[derive(Clone, Debug)]
pub struct UserSongInfo {
    pub play_count: u8,
    pub rating: u8,
}

impl UserSongInfo {
    #[must_use]
    pub const fn default() -> Self {
        Self {
            play_count: 0,
            rating: 0,
        }
    }

    /// Copies info from `other` and merges into `self`:
    /// - Play counts are summed up
    /// - Ratings are averaged, or whichever one is non-zero is used
    pub const fn combine_with(&mut self, other: &UserSongInfo) {
        self.play_count += other.play_count;
        if self.rating == 0 {
            self.rating = other.rating;
        } else if other.rating > 0 {
            self.rating = (self.rating + other.rating) / 2;
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

impl<'s> Song {
    /// Constructs a new `Song` from a `gio::File`
    #[inline]
    #[must_use]
    pub const fn new(file: gio::File) -> Song {
        Song {
            album: None,
            file,
            info: None,
            user_info: UserSongInfo::default(),
            detailed_info: None,
        }
    }
    /// Constructs a new `Song` from a file path
    #[inline]
    #[must_use]
    pub fn new_from_path(file: &str) -> Song {
        Song {
            album: None,
            file: gio::File::for_path(file),
            info: None,
            user_info: UserSongInfo::default(),
            detailed_info: None,
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

        serialize!(
            uri => "uri",
            info.title => "title",
            info.album => "album",
            info.artist => "artist",
            info.album_artist => "album_artist",
            info.track => "track",
            info.disc => "disc",
            info.year => "year",
            info.duration.nseconds() => "duration",
            user_info.play_count => "play_count",
            user_info.rating => "rating",
        )
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

        deserialize!(
            data,
            "uri"<"&str"> => uri,
            "title"<"String"> => info.title,
            "album"<"String"> => info.album,
            "artist"<"String"> => info.artist,
            "album_artist"<"String"> => info.album_artist,
            "track"<"parse"> => info.track,
            "disc"<"parse"> => info.disc,
            "year"<"parse"> => info.year,
            "duration"<"ClockTime"> => info.duration,
            "play_count"<"parse"> => user_info.play_count,
            "rating"<"parse"> => user_info.rating,
        );

        if uri.is_empty() {
            return Err("Could not initialize `uri`".to_string());
        }

        Ok(Song {
            album: None,
            file: gio::File::for_uri(uri),
            info: Some(info),
            user_info,
            detailed_info: None,
        })
    }

    /// Returns a `SongInfoLoader`, which can be used to access information
    /// about the file and song. Tags are loaded on-demand, and remain in
    /// memory until the respective `unload` or `take` method is called.
    #[inline]
    #[must_use]
    pub const fn info(&'s mut self) -> SongInfoLoader<'s> {
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
    detailed_info: &'i mut Option<DetailedSongInfo>,
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
    pub fn take_basic(&mut self) -> SongInfo {
        self.load_basic();
        // SAFETY: `load_basic()` ensures the value is `Some`
        unsafe { self.info.take().unwrap_unchecked() }
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
            album_artist: tag.get_string(&ItemKey::AlbumArtist).map_or_else(
                || tag.artist().unwrap_or_default().to_string(),
                |album_artist| album_artist.to_string(),
            ),
            track: tag.track().unwrap_or_default(),
            disc: tag.disk().unwrap_or(1),
            year: tag.year().unwrap_or_default(),
            #[allow(clippy::cast_possible_truncation)]
            duration: ClockTime::from_mseconds(properties.duration().as_millis() as u64),
        }))
    }

    /// Loads detailed song info if needed, then returns it
    #[inline]
    #[must_use]
    pub fn detailed(&mut self) -> &DetailedSongInfo {
        self.load_detailed();
        // SAFETY: `load_detailed()` ensures the value is `Some`
        unsafe { self.detailed_info.as_ref().unwrap_unchecked() }
    }
    /// Loads detailed song info if needed, and runs the provided closure
    /// if the info had to be loaded
    /// Returns a reference to the loaded info
    #[inline]
    #[must_use]
    pub fn detailed_and<F: FnOnce()>(&mut self, run_if_not_loaded: F) -> &DetailedSongInfo {
        if self.detailed_info.is_none() {
            self.load_detailed();
            run_if_not_loaded();
        }
        // SAFETY: `load_basic()` ensures the value is `Some`
        unsafe { self.detailed_info.as_ref().unwrap_unchecked() }
    }
    /// Loads detailed song info if needed, then returns and unloads it
    #[inline]
    pub fn take_detailed(&mut self) -> DetailedSongInfo {
        self.load_detailed();
        // SAFETY: `load_detailed()` ensures the value is `Some`
        unsafe { self.detailed_info.take().unwrap_unchecked() }
    }
    /// Returns the detailed song info if loaded, but does not load it
    #[inline]
    pub const fn inspect_detailed(&mut self) -> Option<&DetailedSongInfo> {
        self.detailed_info.as_ref()
    }
    /// Loads detailed song info so it is ready to be used later
    /// This method call can be chained
    #[inline]
    pub fn load_detailed(&mut self) {
        if self.detailed_info.is_some() {
            return;
        }
        *self.detailed_info = self
            .load_detailed_from_file()
            .inspect_err(|e| {
                eprintln!(
                    "Problem loading tags (detailed): {:?}: {e}",
                    self.file.path().unwrap_or_default()
                );
            })
            .unwrap_or_else(|_| {
                Some(DetailedSongInfo {
                    lyrics: String::new(),
                    artwork: None,
                })
            });
    }
    /// Unloads detailed song info
    #[inline]
    pub fn unload_detailed(&mut self) {
        *self.detailed_info = None;
    }
    #[inline]
    fn load_detailed_from_file(&mut self) -> Result<Option<DetailedSongInfo>, Box<dyn Error>> {
        if self.tagged.is_none() {
            self.tagged = Some(Probe::open(self.file.path().unwrap())?.read()?);
        }
        let tagged = self.tagged.as_ref().unwrap();
        let tag = tagged
            .primary_tag()
            .or_else(|| tagged.first_tag())
            .ok_or("No tags found")?;
        Ok(Some(DetailedSongInfo {
            lyrics: tag
                .get_string(&ItemKey::Lyrics)
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
