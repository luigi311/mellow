use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct LibraryPage(ObjectSubclass<imp::LibraryPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}
