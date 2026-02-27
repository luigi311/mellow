use adw::subclass::prelude::*;
use core::cell::Cell;
use gtk::CompositeTemplate;
use gtk::glib;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/library_page.ui")]
pub struct LibraryPage {
    #[template_child]
    pub progress_bar: TemplateChild<gtk::ProgressBar>,
    #[template_child]
    pub view_stack: TemplateChild<adw::ViewStack>,
    #[template_child]
    pub ready_stack: TemplateChild<adw::ViewStack>,

    pub needs_refresh: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for LibraryPage {
    const NAME: &str = "MellowLibraryPage";
    type Type = super::LibraryPage;
    type ParentType = adw::NavigationPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for LibraryPage {}
impl WidgetImpl for LibraryPage {}
impl NavigationPageImpl for LibraryPage {}
