use adw::prelude::*;
use gtk::{gio, glib};

use mellow::about;
use mellow::ui::Application;

pub fn main() -> glib::ExitCode {
    glib::set_application_name(about::app_name());
    glib::set_program_name(Some(about::app_name().to_lowercase()));

    register_resources();
    mellow::init_globals();

    let app = Application::new();

    app.set_accels_for_action("window.close", &["<Ctrl>W", "<Ctrl>Q"]);
    app.set_accels_for_action("win.queue_from_disk", &["<Ctrl>O"]);
    // TODO: Ignore shortcut when the overlay is open
    // app.set_accels_for_action("player.play_pause", &["space"]);

    app.run()
}

#[inline]
fn register_resources() {
    #[cfg(feature = "no-meson")]
    gio::resources_register_include!("mellow.gresource").expect("Failed to register resources");

    #[cfg(not(feature = "no-meson"))]
    gio::resources_register(
        &gio::Resource::load(mellow::about::resources_file())
            .expect("Could not load resources file"),
    );
}
