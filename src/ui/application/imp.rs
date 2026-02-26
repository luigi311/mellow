use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;

#[derive(Default)]
pub struct Application {}

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
