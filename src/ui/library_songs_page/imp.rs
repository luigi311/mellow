use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_songs_page.ui")]
pub struct LibrarySongsPage {}

#[glib::object_subclass]
impl ObjectSubclass for LibrarySongsPage {
    const NAME: &str = "MellowLibrarySongsPage";
    type Type = super::LibrarySongsPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for LibrarySongsPage {}
impl WidgetImpl for LibrarySongsPage {}
impl NavigationPageImpl for LibrarySongsPage {}
