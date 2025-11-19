use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;
use gtk::prelude::WidgetExt;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/rating.ui")]
pub struct Rating {
    // #[template_child]
    // stars: TemplateChild<gtk::Box>,
}

impl Rating {
    pub fn init_widgets(&self) {
        let click = gtk::GestureClick::builder()
            .propagation_phase(gtk::PropagationPhase::Capture)
            .build();
        click.connect_released(move |_, idk1, idk2, idk3| {
            dbg!(idk1);
            dbg!(idk2);
            dbg!(idk3);
        });
        self.obj().add_controller(click);
        // self.
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Rating {
    const NAME: &str = "MellowRating";
    type Type = super::Rating;
    type ParentType = gtk::Box;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for Rating {}
impl WidgetImpl for Rating {}
impl BoxImpl for Rating {}
