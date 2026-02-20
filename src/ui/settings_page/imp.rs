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
use crate::ui::settings_page::StartupQueueChoice;

#[derive(Default, CompositeTemplate)]
#[template(resource = "/com/github/userwithaname/Mellow/settings_page.ui")]
pub struct SettingsPage {
    // Playback Settings
    #[template_child]
    pub volume: TemplateChild<gtk::Scale>,
    #[template_child]
    pub gapless: TemplateChild<adw::SwitchRow>,

    // Appearance settings
    #[template_child]
    pub adaptive_colors: TemplateChild<adw::SwitchRow>,
    #[template_child]
    pub color_scheme: TemplateChild<adw::ComboRow>,

    // Directory Settings
    #[template_child]
    pub directory_list: TemplateChild<gtk::ListBox>,

    // Startup Settings
    pub startup_choice: RefCell<StartupQueueChoice>,
    #[template_child]
    pub remember_queue_row: TemplateChild<adw::ExpanderRow>,
    #[template_child]
    pub remember_queue: TemplateChild<gtk::CheckButton>,
    #[template_child]
    pub remember_time: TemplateChild<adw::SwitchRow>,
    #[template_child]
    pub new_queue_row: TemplateChild<adw::ExpanderRow>,
    #[template_child]
    pub new_queue: TemplateChild<gtk::CheckButton>,
    // TODO: Remember shuffle preference and queue source even when disabled?
    #[template_child]
    pub shuffle_queue: TemplateChild<adw::SwitchRow>,
    #[template_child]
    pub queue_source: TemplateChild<adw::ComboRow>,
    #[template_child]
    pub empty_queue: TemplateChild<gtk::CheckButton>,

    pub directories: RefCell<Vec<String>>,

    pub css: OnceCell<gtk::CssProvider>,
    pub style_manager: OnceCell<adw::StyleManager>,
    current_color: Cell<Option<(f64, f64, f64)>>,

    pub style_main: RefCell<Vec<gtk::Widget>>,
    pub style_menu: RefCell<Vec<gtk::Widget>>,
    pub has_style: Cell<bool>,
}

#[gtk::template_callbacks]
impl SettingsPage {
    #[template_callback]
    pub fn handle_set_volume(&self, _: gtk::ScrollType, value: f64) -> glib::Propagation {
        if approx_eq(value, self.volume.value()) {
            return glib::Propagation::Stop;
        }
        (PLAYER_TX.get().expect(EXP_INIT))
            .send(PlayerRequest::SetVolume(value * value))
            .expect(EXP_RX);
        glib::Propagation::Proceed
    }
    #[template_callback]
    pub fn handle_gapless_switch(&self) {
        (PLAYER_TX.get().expect(EXP_INIT))
            .send(PlayerRequest::SetGapless(self.gapless.is_active()))
            .expect(EXP_RX);
    }

    #[template_callback]
    pub fn handle_adaptive_colors_switch(&self) {
        match self.adaptive_colors.is_active() {
            true => self.enable_background_color(),
            false => self.disable_background_color(),
        }
    }

    #[template_callback]
    pub fn handle_theme_dropdown(&self) {
        self.set_theme(match self.color_scheme.selected() {
            0 => adw::ColorScheme::ForceDark,
            1 => adw::ColorScheme::ForceLight,
            2 => adw::ColorScheme::Default,
            _ => unimplemented!(),
        });
    }

    #[template_callback]
    pub fn handle_select_remember_queue(&self) {
        let expanded = self.remember_queue_row.is_expanded();
        if !expanded && !self.remember_queue.is_active() {
            return;
        }
        self.remember_queue.set_active(true);
        self.startup_choice
            .replace(StartupQueueChoice::RestoreQueue);
        self.remember_queue_row.set_expanded(expanded);
        self.new_queue_row.set_expanded(false);
    }
    #[template_callback]
    pub fn handle_select_new_queue(&self) {
        let expanded = self.new_queue_row.is_expanded();
        if !expanded && !self.new_queue.is_active() {
            return;
        }
        self.new_queue.set_active(true);
        self.remember_queue_row.set_expanded(false);
        self.new_queue_row.set_expanded(expanded);
    }
    #[template_callback]
    pub fn handle_select_empty_queue(&self) {
        if self.empty_queue.is_active() {
            return;
        }
        self.empty_queue.set_active(true);
        self.startup_choice.replace(StartupQueueChoice::EmptyQueue);
        self.remember_queue_row.set_expanded(false);
        self.new_queue_row.set_expanded(false);
    }
    #[template_callback]
    pub fn handle_collapse_queue_rows(&self) {
        self.remember_queue_row
            .set_expanded(self.remember_queue.is_active());
        self.new_queue_row.set_expanded(self.new_queue.is_active());
    }
    #[template_callback]
    pub fn handle_update_new_queue_choice(&self) {
        self.startup_choice.replace(
            (1 + self.queue_source.selected() as i32 + (self.shuffle_queue.is_active() as i32 * 3))
                .into(),
        );
    }

    pub fn set_theme(&self, preference: adw::ColorScheme) {
        (self.css.get().expect(EXP_INIT)).set_prefers_color_scheme(match preference {
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
        if self.current_color.get().is_none() || self.has_style.get() {
            return;
        }

        for widget in self.style_main.borrow().iter() {
            widget.add_css_class("color-main");
        }
        for widget in self.style_menu.borrow().iter() {
            widget.add_css_class("color-menu");
        }

        self.has_style.set(true);
    }
    pub fn disable_background_color(&self) {
        if !self.has_style.get() {
            return;
        }

        for widget in self.style_main.borrow().iter() {
            widget.remove_css_class("color-main");
        }
        for widget in self.style_menu.borrow().iter() {
            widget.remove_css_class("color-menu");
        }

        self.has_style.set(false);
    }
    pub fn reset_background_color(&self) {
        if self.css.get().unwrap().prefers_color_scheme() == InterfaceColorScheme::Default {
            self.set_theme(adw::ColorScheme::Default);
        }
        self.current_color.set(None);
        self.disable_background_color();
    }

    /// Returns a linearized color channel
    ///
    /// The input value must be scaled to a 0 to 1 range
    /// (e.g. color as f64 / 255.0)
    #[inline]
    fn srgb_to_linear(c: f64) -> f64 {
        c.powf(2.2)
    }

    /// Sets the interface colors based on the input `r` `g` `b`
    /// values. The colors are processed differently based on the
    /// selected color scheme.
    ///
    /// The input values are expected to be linear rather than sRGB
    pub fn set_background_color(&self, r: f64, g: f64, b: f64) {
        #[inline]
        fn process_color_dark(mut r: f64, mut g: f64, mut b: f64) -> (u8, u8, u8) {
            const SATURATION: f64 = 1.6;

            r = (1.0 - r / 2.0).mul_add(-(1.0 - r / 2.0), 1.0) / 2.0;
            g = (1.0 - g / 2.0).mul_add(-(1.0 - g / 2.0), 1.0) / 2.0;
            b = (1.0 - b / 2.0).mul_add(-(1.0 - b / 2.0), 1.0) / 2.0;

            let lum = lum(r, g, b);

            r = lerp(lum, r, SATURATION);
            g = lerp(lum, g, SATURATION);
            b = lerp(lum, b, SATURATION);

            linear_to_srgb(r, g, b)
        }
        #[inline]
        fn process_color_light(mut r: f64, mut g: f64, mut b: f64) -> (u8, u8, u8) {
            /// Colors below this luminance value will be desaturated for accuracy
            const DESATURATION_THRESHOLD: f64 = 0.1;

            let lum = lum(r, g, b);

            if lum < DESATURATION_THRESHOLD {
                let saturation = lerp(
                    1.0 - (1.0 - lum / DESATURATION_THRESHOLD).powi(3),
                    1.0,
                    0.15,
                );
                r = lerp(lum, r, saturation);
                g = lerp(lum, g, saturation);
                b = lerp(lum, b, saturation);
            }

            // Normalize the color to brighten it without losing saturation
            if lum > 0.0 {
                r /= lum;
                g /= lum;
                b /= lum;
            } else {
                r = 1.0;
                g = 1.0;
                b = 1.0;
            }

            // Scale the normalized color to the target luminance
            let target_lum = lum * lum / 2.0 + 0.5;
            r *= target_lum;
            g *= target_lum;
            b *= target_lum;

            linear_to_srgb(r, g, b)
        }
        #[inline]
        fn process_color_auto(mut r: f64, mut g: f64, mut b: f64) -> ((u8, u8, u8), f64) {
            const SATURATION: f64 = 1.4;

            r = lerp(r, r * r, 0.4);
            g = lerp(g, g * g, 0.4);
            b = lerp(b, b * b, 0.4);

            let lum = lum(r, g, b);
            r = lerp(lum, r, SATURATION);
            g = lerp(lum, g, SATURATION);
            b = lerp(lum, b, SATURATION);

            (linear_to_srgb(r, g, b), lum.powf(1.0 / 2.2))
        }
        /// Color luminance function:
        /// <https://stackoverflow.com/questions/596216/formula-to-determine-perceived-brightness-of-rgb-color/596243#596243>
        #[inline]
        fn lum(r: f64, g: f64, b: f64) -> f64 {
            r.mul_add(0.2126, g.mul_add(0.7152, b * 0.0722))
            // r.mul_add(0.299, g.mul_add(0.587, b * 0.114))
        }
        #[inline]
        fn linear_to_srgb(r: f64, g: f64, b: f64) -> (u8, u8, u8) {
            (
                (r.powf(1.0 / 2.2) * 255.0) as u8,
                (g.powf(1.0 / 2.2) * 255.0) as u8,
                (b.powf(1.0 / 2.2) * 255.0) as u8,
            )
        }

        self.current_color.set(Some((r, g, b)));
        let css = self.css.get().expect(EXP_INIT);
        let (r, g, b) = match css.prefers_color_scheme() {
            InterfaceColorScheme::Dark => process_color_dark(r, g, b),
            InterfaceColorScheme::Light => process_color_light(r, g, b),
            _ => match process_color_auto(r, g, b) {
                (color, lum) if lum < 0.5 => {
                    self.style_manager
                        .get()
                        .unwrap()
                        .set_color_scheme(adw::ColorScheme::ForceDark);
                    color
                }
                (color, _) => {
                    self.style_manager
                        .get()
                        .unwrap()
                        .set_color_scheme(adw::ColorScheme::ForceLight);
                    color
                }
            },
        };

        css.load_from_string(&format!(
            ".color-main {{
                 background-color: rgba({r}, {g}, {b}, 1);
                 border-bottom: 0px none;
                 border-right: 0px none;
                 border-left: 0px none;
                 border-top: 0px none;
             }}
             .color-menu {{
                 background-color: rgba({}, {}, {}, 1);
             }}
            ",
            (r / 2).saturating_sub(4),
            (g / 2).saturating_sub(4),
            (b / 2).saturating_sub(4),
        ));

        self.handle_adaptive_colors_switch();
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
        step_size = step_size.max(5);

        let mut channel = 0u8;
        // Each color component is 4 bytes (u32)
        // FIX: This `step_by` is actually sampling different pixels per color component
        for u32_bytes in image_data.windows(4).step_by(step_size) {
            let c = u32::from_ne_bytes(u32_bytes.try_into().unwrap());
            match channel {
                0 => (),
                1 => b += c as f64 / u32::MAX as f64,
                2 => g += c as f64 / u32::MAX as f64,
                3 => r += c as f64 / u32::MAX as f64,
                _ => unreachable!(),
            }
            channel += 1;
            if channel == 4 {
                channel = 0;
            }
        }

        if channel != 0 {
            eprintln!("FIX: {channel} color channels were sampled more times than others");
        }

        // A failed attempt at a rewrite...
        // let mut num_pixels = 0;
        // // NOTE: `take(8)` is just for debugging purposes
        // for pixel in image_data.chunks(4 * 4).step_by(step_size / 5).take(8) {
        //     assert!(pixel.len() == 16);

        //     r += u32::from_ne_bytes(pixel[4..8].try_into().unwrap()) as f64 / u32::MAX as f64;
        //     g += u32::from_ne_bytes(pixel[8..12].try_into().unwrap()) as f64 / u32::MAX as f64;
        //     b += u32::from_ne_bytes(pixel[12..16].try_into().unwrap()) as f64 / u32::MAX as f64;

        //     num_pixels += 1;

        //     // FIX: For some reason all components have roughly the same value here
        //     dbg!((
        //         u32::from_ne_bytes(pixel[0..4].try_into().unwrap()) as f64 / u32::MAX as f64,
        //         u32::from_ne_bytes(pixel[4..8].try_into().unwrap()) as f64 / u32::MAX as f64,
        //         u32::from_ne_bytes(pixel[8..12].try_into().unwrap()) as f64 / u32::MAX as f64,
        //         u32::from_ne_bytes(pixel[12..16].try_into().unwrap()) as f64 / u32::MAX as f64
        //     ));
        // }

        let num_pixels = image_data.len() / (step_size * 4);
        self.set_background_color(
            Self::srgb_to_linear(r / num_pixels as f64),
            Self::srgb_to_linear(g / num_pixels as f64),
            Self::srgb_to_linear(b / num_pixels as f64),
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
