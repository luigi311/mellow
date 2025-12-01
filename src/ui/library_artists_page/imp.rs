use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_artists_page.ui")]
pub struct LibraryArtistsPage {}

#[glib::object_subclass]
impl ObjectSubclass for LibraryArtistsPage {
    const NAME: &str = "MellowLibraryArtistsPage";
    type Type = super::LibraryArtistsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for LibraryArtistsPage {}
impl WidgetImpl for LibraryArtistsPage {}
impl NavigationPageImpl for LibraryArtistsPage {}
