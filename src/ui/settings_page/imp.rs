use adw::{prelude::*, subclass::prelude::*};
use gtk::CompositeTemplate;
use gtk::glib;
use std::cell::OnceCell;
use std::sync::mpsc;

use crate::approx_eq;
use crate::player::PlayerRequest;

use crate::excuses::{EXP_INIT, EXP_RX};

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/settings_page.ui")]
pub struct SettingsPage {
    #[template_child]
    pub volume: TemplateChild<gtk::Scale>,
    #[template_child]
    pub gapless: TemplateChild<adw::SwitchRow>,
    #[template_child]
    pub directories: TemplateChild<adw::ExpanderRow>,

    pub player_tx: OnceCell<mpsc::SyncSender<PlayerRequest>>,
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
