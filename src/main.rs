use gtk::{gio, glib};
use mellow::about;

#[must_use] // To make Clippy happy
pub fn main() -> glib::ExitCode {
    glib::set_application_name(about::app_name());
    glib::set_program_name(Some(about::app_name().to_lowercase()));

    register_resources();
    mellow::init_globals();

    mellow::ui::Application::run()
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
