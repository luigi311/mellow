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
    pub fn update_progress(&self, progress: Option<f64>) {
        let ui = self.imp();
        match progress {
            Some(progress) => {
                ui.progress_bar.set_fraction(progress);
                ui.view_stack.set_visible_child_name("loading"); // TODO: Optimize?
            }
            None => ui.view_stack.set_visible_child_name("ready"),
        }
    }

    pub fn set_empty(&self, empty: bool) {
        let ui = self.imp();
        match empty {
            false => ui.ready_stack.set_visible_child_name("library"),
            true => ui.ready_stack.set_visible_child_name("empty"),
        }
    }
}
