use std::time::Duration;

pub mod library;
pub mod player;
pub mod ui;

pub use player::{PlayerRequest, PlayerResponse};

pub const APP_NAME: &str = "Mellow";
pub const APP_ID: &str = "com.github.userwithaname.Mellow";

pub fn format_duration(duration: &Duration) -> String {
    let duration = duration.as_secs();
    let seconds = duration % 60;
    format!(
        "{}:{}{seconds}",
        (duration - seconds) / 60,
        if seconds < 10 { "0" } else { "" }
    )
}
