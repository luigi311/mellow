#[cfg(feature = "no-meson")]
fn main() {
    const APP_ID: &str = "com.github.userwithaname.Mellow";
    println!("cargo:rustc-env=APP_ID={APP_ID}");

    let app_name = APP_ID.rsplit_once('.').expect("Invalid APP_ID").1;
    println!("cargo:rustc-env=APP_NAME={app_name}");

    let cargo_toml = std::fs::read_to_string(env!("CARGO_MANIFEST_DIR").to_owned() + "/Cargo.toml")
        .expect("Failed to read Cargo.toml");
    for line in cargo_toml.lines() {
        if line.starts_with("version") {
            let mut version = line.split_once("=").unwrap().1.trim();
            version = &version[1..version.len() - 1];
            println!("cargo:rustc-env=APP_VERSION={version}");
        }
    }

    glib_build_tools::compile_resources(
        &["data/resources"],
        "data/resources/resources.gresource.xml",
        "mellow.gresource",
    );
}

#[cfg(not(feature = "no-meson"))]
fn main() {
    let app_name = option_env!("APP_ID")
        .expect("APP_ID env var not set at compile time")
        .rsplit_once('.')
        .expect("Invalid APP_ID")
        .1;
    println!("cargo:rustc-env=APP_NAME={app_name}");
    // Everything else is done by Meson
}
