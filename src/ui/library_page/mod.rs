use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct LibraryPage(ObjectSubclass<imp::LibraryPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for LibraryPage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibraryPage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }
}
