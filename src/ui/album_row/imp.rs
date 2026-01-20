use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/album_row.ui")]
pub struct AlbumRow {
    #[template_child]
    pub prefix_image: TemplateChild<gtk::Picture>,
}

#[glib::object_subclass]
impl ObjectSubclass for AlbumRow {
    const NAME: &str = "MellowAlbumRow";
    type Type = super::AlbumRow;
    type ParentType = adw::ActionRow;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for AlbumRow {}
impl WidgetImpl for AlbumRow {}
impl ActionRowImpl for AlbumRow {}
impl PreferencesRowImpl for AlbumRow {}
impl ListBoxRowImpl for AlbumRow {}
