use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct LibraryAlbumsPage(ObjectSubclass<imp::LibraryAlbumsPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for LibraryAlbumsPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibraryAlbumsPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }
}
