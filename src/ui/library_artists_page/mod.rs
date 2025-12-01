use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct LibraryArtistsPage(ObjectSubclass<imp::LibraryArtistsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for LibraryArtistsPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibraryArtistsPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }
}
