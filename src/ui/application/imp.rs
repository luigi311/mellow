use adw::subclass::prelude::*;
use gtk::glib;

use core::cell::Cell;
use std::thread::JoinHandle;

#[derive(Default)]
pub struct Application {
    pub library_handle: Cell<Option<JoinHandle<()>>>,
}

#[glib::object_subclass]
impl ObjectSubclass for Application {
    const NAME: &str = "MellowApplication";
    type Type = super::Application;
    type ParentType = adw::Application;
}
impl ObjectImpl for Application {}
impl ApplicationImpl for Application {}
impl AdwApplicationImpl for Application {}
impl GtkApplicationImpl for Application {}
