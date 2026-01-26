use adw::prelude::AdwDialogExt;
use gtk::{License, glib::object::IsA};

const APP_NAME: &str = "Mellow";
const VERSION: &str = "0.1.0";
const APP_ID: Option<&str> = option_env!("APP_ID");
const RESOURCES_FILE: Option<&str> = option_env!("RESOURCES_FILE");

const COPYRIGHT: &str = "© 2025 Iva Kotar";
const LICENSE_TYPE: License = License::Gpl30;
const DEVELOPERS: &[&str] = &["Iva Kotar"];
const DESIGNERS: &[&str] = &["Iva Kotar"];

pub fn show_about_dialog(parent: &impl IsA<gtk::Widget>) {
    let about = adw::AboutDialog::builder()
        .application_icon(app_id())
        .application_name(APP_NAME)
        .issue_url("https://github.com/Userwithaname/mellow/issues/")
        .developers(DEVELOPERS)
        .designers(DESIGNERS)
        .copyright(COPYRIGHT)
        .license_type(LICENSE_TYPE)
        .version(VERSION)
        .build();
    about.present(Some(parent));
}

pub const fn app_name() -> &'static str {
    // TODO: Could this be set using Meson as well?
    APP_NAME
}
pub const fn app_version() -> &'static str {
    // TODO: Could this be set using Meson as well?
    VERSION
}
pub const fn app_id() -> &'static str {
    APP_ID.expect("APP_ID env var not set at compile time")
}
pub const fn resources_file() -> &'static str {
    RESOURCES_FILE.expect("RESOURCES_FILE env var not set at compile time")
}

#[cfg(test)]
mod tests {
    use core::error::Error;
    use gtk::License;
    use std::fs;

    use crate::about::{APP_NAME, LICENSE_TYPE, VERSION};

    #[test]
    fn metadata_consistency() -> Result<(), Box<dyn Error>> {
        let cargo_toml = fs::read_to_string(env!("CARGO_MANIFEST_DIR").to_owned() + "/Cargo.toml")?;
        // TODO: Test Meson configuration (version, app name, etc)
        // TODO: Test app ID for widgets and resources

        let mut app_name = "(none)";
        let mut version = "(none)";
        let mut license = "(none)";

        for line in cargo_toml.lines() {
            if line.starts_with("name") {
                app_name = line.split_once("=").unwrap().1.trim();
                app_name = &app_name[1..app_name.len() - 1];
            } else if line.starts_with("version") {
                version = line.split_once("=").unwrap().1.trim();
                version = &version[1..version.len() - 1];
            } else if line.starts_with("license") {
                license = line.split_once("=").unwrap().1.trim();
                license = &license[1..license.len() - 1];
            }
        }

        assert!(
            app_name.to_lowercase() == APP_NAME.to_lowercase(),
            "APP_NAME: {APP_NAME}\nCargo: {app_name}"
        );
        assert!(
            version.to_lowercase() == VERSION.to_lowercase(),
            "VERSION: {VERSION}\nCargo: {version}"
        );

        match LICENSE_TYPE {
            License::Gpl30 => assert!(
                license == "GPL-3.0",
                "LICENSE_TYPE: GPL-3.0\nCargo: {license}"
            ),
            value => panic!("Cannot test license\nLICENSE_TYPE: {value:?}\nCargo: {license}"),
        }

        Ok(())
    }
}
