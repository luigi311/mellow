use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct LibraryHomePage(ObjectSubclass<imp::LibraryHomePage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for LibraryHomePage {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl LibraryHomePage {
    #[must_use]
    pub fn new() -> Self {
        Object::builder().build()
    }
}
