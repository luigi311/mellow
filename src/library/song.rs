use core::error::Error;
use gio::prelude::*;
use gst::ClockTime;
use gtk::{gdk, gio, glib};
use std::sync::{Arc, Mutex};

use lofty::file::TaggedFile;
use lofty::prelude::*;
use lofty::probe::Probe;

use crate::excuses::EXP_SAFE;
use crate::library::Album;

#[derive(Clone)]
pub struct Song {
    pub album: Option<Arc<Mutex<Album>>>,
    file: gio::File,
    // IDEA: Internal mutability?
    info: Option<SongInfo>,
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
    #[must_use]
    pub const fn new(file: gio::File, info: Option<SongInfo>) -> Song {
        Song {
            album: None,
            file,
            info,
            detailed_info: None,
        }
    }
    #[must_use]
    pub fn new_from_str(file: &str, info: Option<SongInfo>) -> Song {
        Song {
            album: None,
            file: gio::File::for_path(file),
            info,
            detailed_info: None,
        }
    }

    /// Returns a `SongInfoLoader`, which can be used to access information
    /// about the file and song. Tags are loaded on-demand, and remain in
    /// memory until the respective `unload` or `take` method is called.
    #[must_use]
    pub const fn info(&'s mut self) -> SongInfoLoader<'s> {
        SongInfoLoader {
            file: &self.file,
            info: &mut self.info,
            detailed_info: &mut self.detailed_info,
            tagged: None,
        }
    }
}

pub struct SongInfoLoader<'i> {
    file: &'i gio::File,
    info: &'i mut Option<SongInfo>,
    detailed_info: &'i mut Option<DetailedSongInfo>,
    tagged: Option<TaggedFile>,
}

impl SongInfoLoader<'_> {
    /// Retruns the song file URI, which can be used by `GStreamer`
    #[must_use]
    pub fn file_uri(&self) -> String {
        self.file.uri().to_string()
    }
    /// Returns the full song file path
    #[must_use]
    pub fn file_path(&self) -> String {
        self.file.path().unwrap().to_str().unwrap().to_string()
    }
    /// Returns the song filename, including the file extestion
    #[must_use]
    pub fn filename(&self) -> String {
        self.file.basename().map_or_else(
            || "Unknown".to_string(),
            |f| f.to_str().unwrap().to_string(),
        )
    }

    /// Loads basic song info if needed, then returns it
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // Cannot panic
    pub fn basic(&mut self) -> &SongInfo {
        self.load_basic();
        self.info.as_ref().expect(EXP_SAFE)
    }
    /// Loads basic song info if needed, then returns and unloads it
    #[allow(clippy::missing_panics_doc)] // Cannot panic
    pub fn take_basic(&mut self) -> SongInfo {
        self.load_basic();
        self.info.take().expect(EXP_SAFE)
    }
    /// Loads basic song info so it is ready to be used later
    /// This method call can be chained
    pub fn load_basic(&mut self) -> &mut Self {
        if self.info.is_some() {
            return self;
        }
        *self.info = self
            .load_basic_from_file()
            .inspect_err(|e| {
                eprintln!(
                    "Problem loading tags (basic): {:?}: {e}",
                    self.file.path().unwrap_or_default()
                )
            })
            .unwrap_or_else(|_| {
                Some(SongInfo {
                    title: self.filename(),
                    album: String::new(),
                    artist: String::new(),
                    album_artist: String::new(),
                    track: 0,
                    disc: 0,
                    year: 0,
                    duration: ClockTime::default(),
                })
            });
        self
    }
    /// Unloads basic song info
    pub fn unload_basic(&mut self) -> &mut Self {
        *self.info = None;
        self
    }
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
            disc: tag.disk().unwrap_or_default(),
            year: tag.year().unwrap_or_default(),
            #[allow(clippy::cast_possible_truncation)]
            duration: ClockTime::from_mseconds(properties.duration().as_millis() as u64),
        }))
    }

    /// Loads detailed song info if needed, then returns it
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // Cannot panic
    pub fn detailed(&mut self) -> &DetailedSongInfo {
        self.load_detailed();
        self.detailed_info.as_ref().expect(EXP_SAFE)
    }
    /// Loads detailed song info if needed, then returns and unloads it
    #[allow(clippy::missing_panics_doc)] // Cannot panic
    pub fn take_detailed(&mut self) -> DetailedSongInfo {
        self.load_detailed();
        self.detailed_info.take().expect(EXP_SAFE)
    }
    /// Loads detailed song info so it is ready to be used later
    /// This method call can be chained
    pub fn load_detailed(&mut self) -> &mut Self {
        if self.detailed_info.is_some() {
            return self;
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
        self
    }
    /// Unloads detailed song info
    pub fn unload_detailed(&mut self) -> &mut Self {
        *self.detailed_info = None;
        self
    }
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
