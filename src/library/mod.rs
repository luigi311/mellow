pub mod library;

pub mod album;
pub mod artist;
pub mod song;

pub use library::Library;

pub use album::Album;
pub use artist::Artist;
pub use song::{Song, SongInfo};
