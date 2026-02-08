use adw::subclass::prelude::*;
use glib::{Object, object::IsA};
use gtk::{gdk, glib, prelude::WidgetExt};

mod imp;

glib::wrapper! {
    pub struct ItemTile(ObjectSubclass<imp::ItemTile>)
        @extends gtk::Box, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for ItemTile {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ItemTile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> ItemTileBuilder {
        ItemTileBuilder {
            item_tile: Self::default(),
        }
    }

    pub fn set_artwork(&self, artwork: &impl IsA<gdk::Paintable>) {
        self.imp().image.set_paintable(Some(artwork));
    }

    pub fn set_info(&self, title: &str, subtitle: &str) {
        let ui = self.imp();
        ui.title.set_label(title);
        ui.subtitle.set_label(subtitle);
    }
}

pub struct ItemTileBuilder {
    item_tile: ItemTile,
}

impl ItemTileBuilder {
    pub fn artwork(self, artwork: &impl IsA<gdk::Paintable>) -> Self {
        self.item_tile.set_artwork(artwork);
        self
    }

    pub fn info(self, title: &str, subtitle: &str) -> Self {
        self.item_tile.set_info(title, subtitle);
        self
    }

    pub fn image_css_classes(self, classes: &[&str]) -> Self {
        self.item_tile.imp().image.set_css_classes(classes);
        self
    }

    pub fn title_css_classes(self, classes: &[&str]) -> Self {
        self.item_tile.imp().image.set_css_classes(classes);
        self
    }

    pub fn subtitle_css_classes(self, classes: &[&str]) -> Self {
        self.item_tile.imp().image.set_css_classes(classes);
        self
    }

    pub fn build(self) -> ItemTile {
        self.item_tile
    }
}
