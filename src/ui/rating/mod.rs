use adw::subclass::prelude::*;
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
        rating.imp().init_stars();
        rating
    }
}

impl Rating {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_rating(&self, rating: u8) {
        self.imp().set_rating(rating);
    }

    pub fn connect_rating_set<F>(&self, f: F)
    where
        F: Fn(u8) + 'static,
        F: Into<Box<F>>,
    {
        self.imp().on_rating_set.replace(Some(f.into()));
    }
}
