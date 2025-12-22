use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::OnceCell;
use std::sync::mpsc;

use crate::approx_eq;
use crate::excuses::{EXP_INIT, EXP_RX};
use crate::library::LibraryRequest;
use crate::player::PlayerRequest;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/settings_page.ui")]
pub struct SettingsPage {
    // Playback Settings
    #[template_child]
    pub volume: TemplateChild<gtk::Scale>,
    #[template_child]
    pub gapless: TemplateChild<adw::SwitchRow>,
    #[template_child]
    pub remember_queue: TemplateChild<adw::SwitchRow>,

    // Directory Settings
    #[template_child]
    pub directory_list: TemplateChild<gtk::ListBox>,

    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
    pub library_tx: OnceCell<mpsc::SyncSender<LibraryRequest>>,
}

#[gtk::template_callbacks]
impl SettingsPage {
    #[template_callback]
    pub fn handle_set_volume(&self, _: gtk::ScrollType, value: f64) -> glib::Propagation {
        if approx_eq(value, self.volume.value()) {
            return glib::Propagation::Stop;
        }
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetVolume(value * value))
            .expect(EXP_RX);
        glib::Propagation::Proceed
    }
    #[template_callback]
    pub fn handle_gapless_switch(&self) {
        self.player_tx
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetGapless(self.gapless.is_active()))
            .expect(EXP_RX);
    }

    pub fn set_directories(&self, directories: &[String]) {
        self.directory_list.remove_all();
        for (i, directory) in directories.iter().enumerate() {
            let prefix_icon = gtk::Image::builder()
                .icon_name("folder-symbolic")
                .margin_top(8)
                .margin_bottom(8)
                .build();
            let directory_row = adw::ActionRow::builder()
                .title(directory)
                .selectable(true)
                .build();
            directory_row.add_prefix(&prefix_icon);
            let remove_button = gtk::Button::builder()
                .icon_name("window-close-symbolic")
                .margin_top(8)
                .margin_bottom(8)
                .has_tooltip(true)
                .tooltip_text("Remove") // TODO: Support translations
                .css_classes(["flat", "circular"])
                .build();
            remove_button.connect_clicked({
                let library_tx = self.library_tx.get().unwrap().clone();
                move |_| {
                    library_tx
                        .send(LibraryRequest::RemoveLibrary(i))
                        .expect(EXP_RX);
                }
            });
            directory_row.add_suffix(&remove_button);
            self.directory_list.append(&directory_row);
        }
        if directories.is_empty() {
            let add_directory_button = adw::ButtonRow::builder()
                .title("Add Directory")
                .start_icon_name("folder-new-symbolic")
                .action_name("win.add_library")
                .build();
            add_directory_button.add_css_class("suggested-action");
            self.directory_list.append(&add_directory_button);
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SettingsPage {
    const NAME: &str = "MellowSettingsPage";
    type Type = super::SettingsPage;
    type ParentType = adw::PreferencesPage;

    fn class_init(class: &mut Self::Class) {
        class.bind_template();
        class.bind_template_callbacks();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for SettingsPage {}
impl WidgetImpl for SettingsPage {}
impl PreferencesPageImpl for SettingsPage {}
