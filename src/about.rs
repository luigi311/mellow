use adw::prelude::AdwDialogExt;
use gtk::{License, glib::object::IsA};

pub const APP_NAME: &str = "Mellow";
pub const VERSION: &str = "0.1.0";
pub const APP_ID: &str = "com.github.userwithaname.Mellow";

const COPYRIGHT: &str = "© 2025 Iva Kotar";
const LICENSE_TYPE: License = License::Gpl30;
const DEVELOPERS: &[&str] = &["Iva Kotar"];
const DESIGNERS: &[&str] = &["Iva Kotar"];

pub fn show_about_dialog(parent: &impl IsA<gtk::Widget>) {
    let about = adw::AboutDialog::builder()
        .application_icon(APP_ID)
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

#[cfg(test)]
mod tests {
    use core::error::Error;
    use gtk::License;
    use std::fs;

    use crate::about::{APP_NAME, LICENSE_TYPE, VERSION};

    #[test]
    fn metadata_consistency() -> Result<(), Box<dyn Error>> {
        let cargo_toml = fs::read_to_string(env!("CARGO_MANIFEST_DIR").to_owned() + "/Cargo.toml")?;

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
            License::MitX11 => assert!(
                license.to_uppercase() == "MIT",
                "LICENSE_TYPE: MIT\nCargo: {license}"
            ),
            License::Lgpl30 => assert!(
                license.to_uppercase() == "LGPL3",
                "LICENSE_TYPE: LGPL3\nCargo: {license}"
            ),
            License::Gpl30 => assert!(
                license.to_uppercase() == "GPL3",
                "LICENSE_TYPE: GPL3\nCargo: {license}"
            ),
            _ => panic!("Unknown license"),
        }

        Ok(())
    }
}
