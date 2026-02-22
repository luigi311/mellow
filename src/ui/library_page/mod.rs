use adw::subclass::prelude::*;
use gtk::glib;

mod imp;

glib::wrapper! {
    pub struct LibraryPage(ObjectSubclass<imp::LibraryPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl LibraryPage {
    #[inline]
    pub fn update_progress(&self, progress: Option<f64>) {
        match progress {
            Some(progress) => {
                let ui = self.imp();
                ui.progress_bar.set_fraction(progress);
                ui.view_stack.set_visible_child_name("loading"); // TODO: Optimize?
            }
            None => self.imp().view_stack.set_visible_child_name("ready"),
        }
    }

    #[inline]
    pub fn set_empty(&self, empty: bool) {
        match empty {
            false => self.imp().ready_stack.set_visible_child_name("library"),
            true => self.imp().ready_stack.set_visible_child_name("empty"),
        }
    }
}
