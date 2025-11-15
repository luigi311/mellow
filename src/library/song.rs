use core::error::Error;
use gio::prelude::*;
use gst::ClockTime;
use gtk::{gdk::Texture, gio, glib};
use std::sync::Arc;

use lofty::file::TaggedFile;
use lofty::prelude::*;
use lofty::probe::Probe;

#[derive(Clone)]
pub struct Song {
    pub file: gio::File,
    pub album: Option<usize>,
    pub info: Option<Arc<SongInfo>>,
    pub detailed_info: Option<Arc<DetailedSongInfo>>,
}

pub struct SongInfo {
    pub title: String,
    pub album: String,
    pub artist: String,
    pub album_artist: String,
    pub track: String,
    pub year: String,
    pub lyrics: String,
    pub duration: ClockTime,
    // TODO: Move memory-heavy into `Song::detailed_info`,
    // so they can be assigned on-demand
    pub artwork: Option<Texture>,
}

// TODO: Move memory-heavy fields into here
/// Fields which do not need to be held in memory at all times
pub struct DetailedSongInfo {
    pub lyrics: String,
    pub artwork: Option<Texture>,
}

impl<'a> Song {
    pub fn new(file: &str, album: Option<usize>) -> Result<Song, Box<dyn Error>> {
        Ok(Song {
            file: gio::File::for_path(file),
            album,
            info: None,
            detailed_info: None,
        })
    }

    #[must_use]
    pub fn file_uri(&self) -> String {
        self.file.uri().to_string()
    }

    #[must_use]
    pub fn filename(&self) -> String {
        self.file.basename().map_or_else(
            || "Unknown".to_string(),
            |f| f.to_str().unwrap().to_string(),
        )
    }

    /// Returns a `SongInfoLoader`, which can be used to access the
    /// song file tags. Loaded info is assigned to `self`.
    pub fn info(&'a mut self) -> SongInfoLoader<'a> {
        // let info = self.info.take();
        // let detailed_info = self.detailed_info.take();

        SongInfoLoader {
            song: self,
            // info,
            // detailed_info,
            tagged: None,
        }
    }
}

pub struct SongInfoLoader<'a> {
    song: &'a mut Song,
    // info: Option<Arc<SongInfo>>,
    // detailed_info: Option<Arc<DetailedSongInfo>>,
    tagged: Option<TaggedFile>,
}

impl<'a> SongInfoLoader<'a> {
    pub fn basic(&mut self) -> &Arc<SongInfo> {
        self.load_basic();
        self.song.info.as_ref().unwrap()
    }
    pub fn load_basic(&mut self) -> &mut Self {
        if self.song.info.is_some() {
            return self;
        }
        println!("Loading basic song info for {}...", self.song.filename());
        self.song.info = self
            .load_basic_from_file()
            .inspect_err(|e| eprintln!("Could not read song properties:\n{e}"))
            .unwrap_or_else(|_| {
                Some(Arc::new(SongInfo {
                    title: self.song.filename(),
                    album: String::new(),
                    artist: String::new(),
                    album_artist: String::new(),
                    track: String::new(),
                    year: String::new(),
                    lyrics: String::new(),
                    duration: ClockTime::default(),
                    artwork: None,
                }))
            });
        self
    }
    pub fn detailed(&mut self) -> &Arc<DetailedSongInfo> {
        self.load_detailed();
        self.song.detailed_info.as_ref().unwrap()
    }
    pub fn load_detailed(&mut self) -> &mut Self {
        if self.song.detailed_info.is_some() {
            return self;
        }
        println!("Loading detailed song info for {}...", self.song.filename());
        self.song.detailed_info = self
            .load_detailed_from_file()
            .inspect_err(|e| eprintln!("Could not read song properties:\n{e}"))
            .unwrap_or_else(|_| {
                Some(Arc::new(DetailedSongInfo {
                    lyrics: String::new(),
                    artwork: None,
                }))
            });
        self
    }
    fn load_basic_from_file(&mut self) -> Result<Option<Arc<SongInfo>>, Box<dyn Error>> {
        if self.tagged.is_none() {
            self.tagged = Some(Probe::open(self.song.file.path().unwrap())?.read()?);
        }
        let tagged = self.tagged.as_ref().unwrap();
        let tag = tagged
            .primary_tag()
            .or_else(|| tagged.first_tag())
            .ok_or("No tags found")?;
        let properties = tagged.properties();

        Ok(Some(Arc::new(SongInfo {
            title: tag.title().map_or_else(
                || self.song.filename(),
                |title| match title.trim().is_empty() {
                    true => self.song.filename(),
                    false => title.to_string(),
                },
            ),
            album: tag.album().unwrap_or_default().to_string(),
            artist: tag.artist().unwrap_or_default().to_string(),
            album_artist: tag
                .get_string(&ItemKey::AlbumArtist)
                .unwrap_or_default()
                .to_string(),
            track: tag.track().unwrap_or_default().to_string(),
            year: tag.year().unwrap_or_default().to_string(),
            lyrics: tag
                .get_string(&ItemKey::Lyrics)
                .unwrap_or_default()
                .to_string(),
            #[allow(clippy::cast_possible_truncation)]
            duration: ClockTime::from_mseconds(properties.duration().as_millis() as u64),
            // TODO: Look for a `cover` file in the song directroy
            artwork: if tag.picture_count() > 0 {
                Some(Texture::from_bytes(&glib::Bytes::from(
                    tag.pictures()[0].data(),
                ))?)
            } else {
                None
            },
        })))
    }
    fn load_detailed_from_file(&mut self) -> Result<Option<Arc<DetailedSongInfo>>, Box<dyn Error>> {
        if self.tagged.is_none() {
            self.tagged = Some(Probe::open(self.song.file.path().unwrap())?.read()?);
        }
        let tagged = self.tagged.as_ref().unwrap();
        let tag = tagged
            .primary_tag()
            .or_else(|| tagged.first_tag())
            .ok_or("No tags found")?;
        Ok(Some(Arc::new(DetailedSongInfo {
            lyrics: tag
                .get_string(&ItemKey::Lyrics)
                .unwrap_or_default()
                .to_string(),
            // TODO: Look for a `cover` file in the song directroy
            artwork: if tag.picture_count() > 0 {
                Some(Texture::from_bytes(&glib::Bytes::from(
                    tag.pictures()[0].data(),
                ))?)
            } else {
                None
            },
        })))
    }
}

// impl<'a> Drop for SongInfoLoader<'a> {
//     fn drop(&mut self) {
//         self.song.info = self.info.take();
//         self.song.detailed_info = self.detailed_info.take();
//     }
// }
