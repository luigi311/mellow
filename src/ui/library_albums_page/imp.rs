use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_albums_page.ui")]
pub struct LibraryAlbumsPage {}

#[glib::object_subclass]
impl ObjectSubclass for LibraryAlbumsPage {
    const NAME: &str = "MellowLibraryAlbumsPage";
    type Type = super::LibraryAlbumsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for LibraryAlbumsPage {}
impl WidgetImpl for LibraryAlbumsPage {}
impl NavigationPageImpl for LibraryAlbumsPage {}
