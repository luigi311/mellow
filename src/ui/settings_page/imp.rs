use adw::{prelude::*, subclass::prelude::*};
use gtk::{CompositeTemplate, InterfaceColorScheme};
use gtk::{gdk, glib};
use std::cell::{Cell, OnceCell, RefCell};

use crate::approx_eq;
use crate::excuses::{EXP_INIT, EXP_RX};
use crate::lerp;
use crate::library::LIBRARY_TX;
use crate::library::LibraryRequest;
use crate::player::PLAYER_TX;
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

    // Appearance settings
    #[template_child]
    pub theme: TemplateChild<adw::ComboRow>,
    #[template_child]
    pub match_colors: TemplateChild<adw::SwitchRow>,

    // Directory Settings
    #[template_child]
    pub directory_list: TemplateChild<gtk::ListBox>,

    pub directories: RefCell<Vec<String>>,

    pub css: OnceCell<gtk::CssProvider>,
    pub style_manager: OnceCell<adw::StyleManager>,
    current_color: Cell<Option<(f64, f64, f64)>>,

    pub bottom_bar: OnceCell<gtk::Box>,
    pub sheet: OnceCell<adw::BottomSheet>,
}

#[gtk::template_callbacks]
impl SettingsPage {
    #[template_callback]
    pub fn handle_set_volume(&self, _: gtk::ScrollType, value: f64) -> glib::Propagation {
        if approx_eq(value, self.volume.value()) {
            return glib::Propagation::Stop;
        }
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetVolume(value * value))
            .expect(EXP_RX);
        glib::Propagation::Proceed
    }
    #[template_callback]
    pub fn handle_gapless_switch(&self) {
        PLAYER_TX
            .get()
            .expect(EXP_INIT)
            .send(PlayerRequest::SetGapless(self.gapless.is_active()))
            .expect(EXP_RX);
    }

    #[template_callback]
    pub fn handle_match_colors_switch(&self) {
        match self.match_colors.is_active() {
            true => self.enable_background_color(),
            false => self.disable_background_color(),
        }
    }

    #[template_callback]
    pub fn handle_theme_dropdown(&self) {
        println!("TODO");
    }

    pub fn set_theme_preference(&self, preference: adw::ColorScheme) {
        self.css
            .get()
            .unwrap()
            .set_prefers_color_scheme(match preference {
                adw::ColorScheme::ForceDark | adw::ColorScheme::PreferDark => {
                    InterfaceColorScheme::Dark
                }
                adw::ColorScheme::ForceLight | adw::ColorScheme::PreferLight => {
                    InterfaceColorScheme::Light
                }
                _ => InterfaceColorScheme::Default,
            });
        self.style_manager
            .get()
            .unwrap()
            .set_color_scheme(preference);
        let color = self.current_color.get();
        match color {
            Some((r, g, b)) => self.set_background_color(r, g, b),
            None => self.disable_background_color(),
        }
    }

    pub fn enable_background_color(&self) {
        if self.current_color.get().is_none()
            || self.sheet.get().expect(EXP_INIT).has_css_class("window")
        {
            return;
        }

        self.sheet.get().expect(EXP_INIT).add_css_class("window");
        self.bottom_bar
            .get()
            .expect(EXP_INIT)
            .add_css_class("window");
    }
    pub fn disable_background_color(&self) {
        if !self.sheet.get().expect(EXP_INIT).has_css_class("window") {
            return;
        }

        self.sheet.get().expect(EXP_INIT).remove_css_class("window");
        self.bottom_bar
            .get()
            .expect(EXP_INIT)
            .remove_css_class("window");
    }
    pub fn reset_background_color(&self) {
        self.current_color.set(None);
        self.disable_background_color();
    }

    pub fn set_background_color(&self, r: f64, g: f64, b: f64) {
        fn process_color_dark(mut r: f64, mut g: f64, mut b: f64) -> (u8, u8, u8) {
            const SATURATION: f64 = 2.0;

            r = 1.0 - (1.0 - r / 2.0).powi(2);
            g = 1.0 - (1.0 - g / 2.0).powi(2);
            b = 1.0 - (1.0 - b / 2.0).powi(2);

            let lum = (r * 0.2126) + (g * 0.7152) + (b * 0.0722);

            r = lerp(lum, r, SATURATION);
            g = lerp(lum, g, SATURATION);
            b = lerp(lum, b, SATURATION);

            (
                (r * 255.0 / 2.0) as u8,
                (g * 255.0 / 2.0) as u8,
                (b * 255.0 / 2.0) as u8,
            )
        }
        fn process_color_light(mut r: f64, mut g: f64, mut b: f64) -> (u8, u8, u8) {
            const SATURATION: f64 = 2.5;

            r = 2.0 - (1.0 - r / 2.0).powi(3);
            g = 2.0 - (1.0 - g / 2.0).powi(3);
            b = 2.0 - (1.0 - b / 2.0).powi(3);

            let lum = (r * 0.2126) + (g * 0.7152) + (b * 0.0722);

            r = lerp(lum, r, SATURATION);
            g = lerp(lum, g, SATURATION);
            b = lerp(lum, b, SATURATION);

            (
                (r * 255.0 * 0.57143) as u8,
                (g * 255.0 * 0.57143) as u8,
                (b * 255.0 * 0.57143) as u8,
            )
        }

        dbg!((r, g, b));

        self.current_color.set(Some((r, g, b)));
        let css = self.css.get().expect(EXP_INIT);
        let (r, g, b) = match css.prefers_color_scheme() {
            InterfaceColorScheme::Dark => process_color_dark(r, g, b),
            InterfaceColorScheme::Light => process_color_light(r, g, b),
            _ => todo!("TODO: Use light or dark based on system theme"),
        };

        dbg!((r, g, b));

        css.load_from_string(&format!(
            ".window {{
                 background-color: rgba({r}, {g}, {b}, 1);
                 border-bottom: 0px none;
                 border-right: 0px none;
                 border-left: 0px none;
                 border-top: 0px none;
             }}"
        ));

        self.handle_match_colors_switch();
    }

    pub fn set_background_from_artwork(&self, artwork: &gdk::Texture) {
        let mut r = 0.0;
        let mut g = 0.0;
        let mut b = 0.0;

        // ARGB32
        let mut image_data = vec![0u8; (artwork.width() * artwork.height()) as usize * 4];
        artwork.download(&mut image_data, 4 * artwork.width() as usize);

        // Pixels will be skipped to match the below target resolution
        const SAMPLE_RES: usize = 128;
        let mut step_size = image_data.len() / (SAMPLE_RES * SAMPLE_RES * 4);
        step_size -= step_size % 4;
        step_size += 1;

        let mut component = 0u8;
        // Each color component is 4 bytes (u32)
        for u32_bytes in image_data.windows(4).step_by(step_size) {
            let c = u32::from_ne_bytes(u32_bytes.try_into().unwrap());
            match component {
                0 => (),
                1 => b += c as f64 / u32::MAX as f64,
                2 => g += c as f64 / u32::MAX as f64,
                3 => r += c as f64 / u32::MAX as f64,
                _ => unreachable!(),
            }
            component += 1;
            if component == 4 {
                component = 0;
            }
        }

        let num_pixels = image_data.len() / (step_size * 4);
        self.set_background_color(
            r / num_pixels as f64,
            g / num_pixels as f64,
            b / num_pixels as f64,
        );
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
                // .activatable(true)
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
            let library_tx = LIBRARY_TX.get().unwrap().clone();
            remove_button.connect_clicked(move |_| {
                library_tx
                    .send(LibraryRequest::RemoveLibrary(i))
                    .expect(EXP_RX);
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
        self.directories.replace(directories.into());
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
