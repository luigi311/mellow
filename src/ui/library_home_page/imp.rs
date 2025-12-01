use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_home_page.ui")]
pub struct LibraryHomePage {}

#[glib::object_subclass]
impl ObjectSubclass for LibraryHomePage {
    const NAME: &str = "MellowLibraryHomePage";
    type Type = super::LibraryHomePage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for LibraryHomePage {}
impl WidgetImpl for LibraryHomePage {}
impl NavigationPageImpl for LibraryHomePage {}
