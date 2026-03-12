use adw::{prelude::*, subclass::prelude::*};
use core::cell::{Cell, OnceCell, RefCell};
use gtk::CompositeTemplate;
use gtk::glib;

use crate::excuses::EXP_INIT;

const NUM_STARS: u8 = 5;
const DEFAULT_STAR_SIZE: i32 = 16;
const SMALL_STAR_SIZE: i32 = 14;
const SMALL_STAR_MARGIN: i32 = (DEFAULT_STAR_SIZE - SMALL_STAR_SIZE) / 2;

type RateFn = RefCell<Option<Box<dyn Fn(u8)>>>;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/rating.ui")]
pub struct Rating {
    stars: OnceCell<Box<[gtk::Image]>>,
    pub rating: Cell<u8>,
    pub on_rating_set: RateFn,
}

impl Rating {
    /// Initializes the widget controllers
    #[inline]
    fn init_stars(&self) {
        let mut stars = Vec::new();
        for _ in 0..NUM_STARS {
            let star = gtk::Image::builder()
                .icon_name("starred-symbolic")
                .css_classes(["dimmed"])
                .build();
            self.obj().append(&star);
            stars.push(star);
        }
        let _ = self.stars.set(stars.into());

        let motion = gtk::EventControllerMotion::builder()
            .propagation_phase(gtk::PropagationPhase::Capture)
            .build();
        motion.connect_motion(glib::clone!(
            #[weak(rename_to=rating)]
            self,
            move |_, pos_x, _| {
                rating.preview_rating(rating.rating.get(), rating.pixels_to_rating(pos_x));
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
                let new_rating = rating.pixels_to_rating(pos_x);
                rating.set_rating(match new_rating == rating.rating.get() {
                    false => new_rating,
                    true => 0,
                });
            }
        ));
        self.obj().add_controller(click);
    }

    /// Sets the rating to the given value
    #[inline]
    pub fn set_rating(&self, rating: u8) {
        self.rating.set(rating);
        self.show_rating(rating);
        if let Some(on_rating_set) = self.on_rating_set.borrow().as_ref() {
            on_rating_set(rating);
        }
    }

    /// Highlights the stars to match the `rating`
    #[inline]
    pub fn show_rating(&self, rating: u8) {
        let stars = self.stars.get().expect(EXP_INIT);
        for i in 0..rating {
            let star = &stars[i as usize];
            star.remove_css_class("dimmed");
            star.set_pixel_size(DEFAULT_STAR_SIZE);
            star.set_margin_start(0);
            star.set_margin_end(0);
        }
        for i in rating..NUM_STARS {
            let star = &stars[i as usize];
            star.add_css_class("dimmed");
            star.set_pixel_size(DEFAULT_STAR_SIZE);
            star.set_margin_start(0);
            star.set_margin_end(0);
        }
    }

    /// Highlights the stars to match the `preview` rating,
    /// and shows inactive stars as either smaller or regular
    /// sized, to show the previous `rating`
    #[inline]
    pub fn preview_rating(&self, rating: u8, preview: u8) {
        let stars = self.stars.get().expect(EXP_INIT);
        let rating = rating.max(preview);
        for i in 0..preview {
            let star = &stars[i as usize];
            star.remove_css_class("dimmed");
            star.set_pixel_size(DEFAULT_STAR_SIZE);
            star.set_margin_start(0);
            star.set_margin_end(0);
        }
        for i in preview..rating {
            let star = &stars[i as usize];
            star.add_css_class("dimmed");
            star.set_pixel_size(DEFAULT_STAR_SIZE);
            star.set_margin_start(0);
            star.set_margin_end(0);
        }
        for i in rating..NUM_STARS {
            let star = &stars[i as usize];
            star.add_css_class("dimmed");
            star.set_pixel_size(SMALL_STAR_SIZE);
            star.set_margin_start(SMALL_STAR_MARGIN);
            star.set_margin_end(SMALL_STAR_MARGIN);
        }
    }

    /// Returns the rating at the given `pos_x` pixel position
    pub fn pixels_to_rating(&self, pos_x: f64) -> u8 {
        let spacing = self.obj().spacing() as f64;
        let star_width = DEFAULT_STAR_SIZE as f64 + spacing;
        (((pos_x + spacing / 2.0) / star_width) as u8 + 1).min(5)
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
        self.obj().set_cursor_from_name(Some("pointer"));
    }
}
impl WidgetImpl for Rating {}
impl BoxImpl for Rating {}
