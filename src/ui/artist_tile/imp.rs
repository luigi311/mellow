use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::Cell;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/artist_tile.ui")]
pub struct ArtistTile {
    #[template_child]
    pub artist_image: TemplateChild<gtk::Picture>,
    #[template_child]
    pub artist: TemplateChild<gtk::Label>,
    #[template_child]
    pub num_albums: TemplateChild<gtk::Label>,

    pub index: Cell<u64>,
}

#[glib::object_subclass]
impl ObjectSubclass for ArtistTile {
    const NAME: &str = "MellowArtistTile";
    type Type = super::ArtistTile;
    type ParentType = gtk::Box;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ArtistTile {}
impl WidgetImpl for ArtistTile {}
impl BoxImpl for ArtistTile {}
