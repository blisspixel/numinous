//! Embed native package resources for the windowed app.

fn main() {
    const ICON: &str = "../../assets/logo.ico";
    println!("cargo:rerun-if-changed={ICON}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut resource = winresource::WindowsResource::new();
    resource.set_icon(ICON);
    resource
        .compile()
        .expect("compile the Numinous Windows icon resource");
}
