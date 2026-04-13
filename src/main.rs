use gtk::glib;

#[must_use] // To make Clippy happy
pub fn main() -> glib::ExitCode {
    mellow::init_globals();
    mellow::ui::Application::run()
}
