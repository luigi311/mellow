use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/queue_row.ui")]
pub struct QueueRow {
    #[template_child]
    pub prefix_image: TemplateChild<gtk::Picture>,
}

#[glib::object_subclass]
impl ObjectSubclass for QueueRow {
    const NAME: &str = "MellowQueueRow";
    type Type = super::QueueRow;
    type ParentType = adw::ActionRow;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for QueueRow {}
impl WidgetImpl for QueueRow {}
impl ActionRowImpl for QueueRow {}
impl PreferencesRowImpl for QueueRow {}
impl ListBoxRowImpl for QueueRow {}
