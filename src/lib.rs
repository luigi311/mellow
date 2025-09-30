use std::time::Duration;

pub mod library;
pub mod player;
pub mod ui_gtk;

pub use library::*;
pub use player::*;
pub use ui_gtk::*;

pub fn format_duration(duration: &Duration) -> String {
    let duration = duration.as_secs();
    let seconds = duration % 60;
    format!(
        "{}:{}{seconds}",
        (duration - seconds) / 60,
        if seconds < 10 { "0" } else { "" }
    )
}
