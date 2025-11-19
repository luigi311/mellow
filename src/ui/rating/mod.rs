use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct Rating(ObjectSubclass<imp::Rating>)
        @extends gtk::Box, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for Rating {
    fn default() -> Self {
        let rating: Rating = Object::builder().build();
        rating.imp().init_widgets();
        rating
    }
}

impl Rating {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}
