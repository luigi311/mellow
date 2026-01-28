use adw::{prelude::*, subclass::prelude::*};
use glib::{Object, object::IsA};
use gtk::{gdk, glib};

mod imp;

glib::wrapper! {
    pub struct ItemRow(ObjectSubclass<imp::ItemRow>)
        @extends gtk::Box, gtk::Widget,
        @implements
            gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for ItemRow {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl ItemRow {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> ItemRowBuilder {
        ItemRowBuilder {
            item_row: Self::default(),
        }
    }

    pub fn set_artwork(&self, image: &impl IsA<gdk::Paintable>) {
        self.imp().image.set_paintable(Some(image));
    }

    pub fn set_info(&self, title: &str, subtitle: &str) {
        let ui = self.imp();
        ui.title.set_label(title);
        ui.subtitle.set_label(subtitle);
    }
}

pub struct ItemRowBuilder {
    item_row: ItemRow,
}

impl ItemRowBuilder {
    pub fn artwork(self, artwork: &impl IsA<gdk::Paintable>) -> Self {
        self.item_row.set_artwork(artwork);
        self
    }

    pub fn titles(self, album: &str, artist: &str) -> Self {
        self.item_row.set_info(album, artist);
        self
    }

    pub fn image_css_classes(self, classes: &[&str]) -> Self {
        self.item_row.imp().image.set_css_classes(classes);
        self
    }

    pub fn title_css_classes(self, classes: &[&str]) -> Self {
        self.item_row.imp().image.set_css_classes(classes);
        self
    }

    pub fn subtitle_css_classes(self, classes: &[&str]) -> Self {
        self.item_row.imp().image.set_css_classes(classes);
        self
    }

    pub fn build(self) -> ItemRow {
        self.item_row
    }
}
