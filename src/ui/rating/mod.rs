use adw::subclass::prelude::*;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct Rating(ObjectSubclass<imp::Rating>)
        @extends gtk::Box, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Rating {
    /// Returns the current rating assigned to the widget
    pub fn get_rating(&self) -> u8 {
        self.imp().rating.get()
    }

    /// Sets the rating and runs the `on_rating_set` closure
    pub fn set_rating(&self, rating: u8) {
        self.imp().set_rating(rating);
    }

    /// Sets the rating without running the `on_rating_set` closure
    pub fn set_rating_silent(&self, rating: u8) {
        let ui = self.imp();
        ui.rating.set(rating);
        ui.show_rating(rating);
    }

    /// Connects a closure to run when a new rating is set
    pub fn connect_rating_set<F>(&self, f: F)
    where
        F: Fn(u8) + 'static,
        F: Into<Box<F>>,
    {
        self.imp().on_rating_set.replace(Some(f.into()));
    }
}
