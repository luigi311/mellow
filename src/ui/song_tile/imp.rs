use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::Cell;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/song_tile.ui")]
pub struct SongTile {
    #[template_child]
    pub album_cover: TemplateChild<gtk::Picture>,
    #[template_child]
    pub title: TemplateChild<gtk::Label>,
    #[template_child]
    pub artist: TemplateChild<gtk::Label>,

    pub index: Cell<u64>,
}

#[glib::object_subclass]
impl ObjectSubclass for SongTile {
    const NAME: &str = "MellowSongTile";
    type Type = super::SongTile;
    type ParentType = gtk::Box;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for SongTile {}
impl WidgetImpl for SongTile {}
impl BoxImpl for SongTile {}
