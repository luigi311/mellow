use adw::subclass::prelude::*;
use core::cell::RefCell;
use gtk::CompositeTemplate;
use gtk::glib;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/song_row.ui")]
pub struct SongRow {
    #[template_child]
    pub prefix_image: TemplateChild<gtk::Picture>,
    #[template_child]
    pub suffix_label: TemplateChild<gtk::Label>,

    pub bindings: RefCell<Vec<glib::Binding>>,
}

#[glib::object_subclass]
impl ObjectSubclass for SongRow {
    const NAME: &str = "MellowSongRow";
    type Type = super::SongRow;
    type ParentType = adw::ActionRow;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for SongRow {}
impl WidgetImpl for SongRow {}
impl ActionRowImpl for SongRow {}
impl PreferencesRowImpl for SongRow {}
impl ListBoxRowImpl for SongRow {}
