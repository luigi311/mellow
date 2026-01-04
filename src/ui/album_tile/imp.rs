use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::Cell;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/album_tile.ui")]
pub struct AlbumTile {
    #[template_child]
    pub album_cover: TemplateChild<gtk::Picture>,
    #[template_child]
    pub album: TemplateChild<gtk::Label>,
    #[template_child]
    pub artist: TemplateChild<gtk::Label>,

    pub index: Cell<u64>,
}

#[glib::object_subclass]
impl ObjectSubclass for AlbumTile {
    const NAME: &str = "MellowAlbumTile";
    type Type = super::AlbumTile;
    type ParentType = gtk::Box;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for AlbumTile {}
impl WidgetImpl for AlbumTile {}
impl BoxImpl for AlbumTile {}
