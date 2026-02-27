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
        let ui = self.imp();
        if let Some(progress) = progress {
            ui.progress_bar.set_fraction(progress);
            if ui.needs_refresh.get() {
                ui.view_stack.set_visible_child_name("loading");
                ui.needs_refresh.set(false);
            }
        } else {
            ui.view_stack.set_visible_child_name("ready");
            ui.needs_refresh.set(true);
        }
    }

    #[inline]
    pub fn set_empty(&self, empty: bool) {
        match empty {
            false => self.imp().ready_stack.set_visible_child_name("library"),
            true => self.imp().ready_stack.set_visible_child_name("empty"),
        }
        self.imp().needs_refresh.set(true);
    }
}

pub enum SubpageType {
    Song,
    Album,
    Artist,
}
