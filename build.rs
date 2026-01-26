#[cfg(feature = "no-meson")]
fn main() {
    const APP_ID: &str = "com.github.userwithaname.Mellow";
    println!("cargo:rustc-env=APP_ID={APP_ID}");
    glib_build_tools::compile_resources(
        &["data/resources"],
        "data/resources/resources.gresource.xml",
        "mellow.gresource",
    );
}

#[cfg(not(feature = "no-meson"))]
fn main() {}
