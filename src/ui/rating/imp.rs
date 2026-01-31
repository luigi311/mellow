use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::{Cell, OnceCell, RefCell};

use crate::excuses::{EXP_INIT, INIT_ERR};

const NUM_STARS: u8 = 5;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/rating.ui")]
pub struct Rating {
    stars: OnceCell<Box<[gtk::Image]>>,
    pub rating: Cell<u8>,
    pub on_rating_set: RefCell<Option<Box<dyn Fn(u8)>>>,
}

impl Rating {
    pub fn init_stars(&self) {
        let mut stars = Vec::new();
        for _ in 0..NUM_STARS {
            let star = gtk::Image::builder()
                .icon_name("starred-symbolic")
                .css_classes(["dimmed"])
                .height_request(32)
                .margin_start(4)
                .margin_end(4)
                .build();
            self.obj().append(&star);
            stars.push(star);
        }
        self.stars.set(stars.into()).expect(INIT_ERR);

        let motion = gtk::EventControllerMotion::builder()
            .propagation_phase(gtk::PropagationPhase::Capture)
            .build();
        motion.connect_motion(glib::clone!(
            #[weak(rename_to=rating)]
            self,
            move |_, pos_x, _| {
                rating.show_rating(rating.pixels_to_rating(pos_x));
            }
        ));
        motion.connect_leave(glib::clone!(
            #[weak(rename_to=rating)]
            self,
            move |_| rating.show_rating(rating.rating.get())
        ));
        self.obj().add_controller(motion);

        let click = gtk::GestureClick::builder()
            .propagation_phase(gtk::PropagationPhase::Capture)
            .build();
        click.connect_released(glib::clone!(
            #[weak(rename_to=rating)]
            self,
            move |_, _, pos_x, _| {
                rating.set_rating(rating.pixels_to_rating(pos_x));
            }
        ));
        self.obj().add_controller(click);
    }

    pub fn set_rating(&self, rating: u8) {
        self.rating.set(rating);
        self.show_rating(rating);
        if let Some(on_rating_set) = self.on_rating_set.borrow().as_ref() {
            on_rating_set(rating);
        }
    }

    pub fn show_rating(&self, rating: u8) {
        let stars = self.stars.get().expect(EXP_INIT);
        for i in 0..rating {
            stars[i as usize].remove_css_class("dimmed");
        }
        for i in rating..NUM_STARS {
            stars[i as usize].add_css_class("dimmed");
        }
    }

    pub fn pixels_to_rating(&self, pos_x: f64) -> u8 {
        let star = &self.stars.get().as_ref().expect(EXP_INIT)[0];
        let star_width = (star.width() + star.margin_start() + star.margin_end()) as f64;
        let spacing = self.obj().spacing() as f64;
        (pos_x / (star_width + spacing) - spacing / 2.0) as u8 + 1
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
impl ObjectImpl for Rating {
    fn constructed(&self) {
        self.init_stars();
    }
}
impl WidgetImpl for Rating {}
impl BoxImpl for Rating {}
