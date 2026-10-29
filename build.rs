use slint_build::{CompilerConfiguration, DefaultTranslationContext};

const ICON: &str = "assets/logo.ico";

fn main() {
    // Translations live in lang/<code>/LC_MESSAGES/avalio-lite.po and are bundled into the exe.
    // No per-component context, so each English string is translated once wherever it appears.
    // Debug builds (tests, coverage) keep element ids, which the UI tests find elements by.
    let debug = std::env::var("PROFILE").is_ok_and(|profile| profile == "debug");
    let config = CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_bundled_translations("lang")
        .with_default_translation_context(DefaultTranslationContext::None)
        .with_debug_info(debug);
    slint_build::compile_with_config("ui/app-window.slint", config).expect("Slint UI failed to compile");

    // The exe's own icon, shown by Explorer, the Start menu and the uninstall entry. A build
    // without it is broken, so a failure stops the build.
    println!("cargo:rerun-if-changed={ICON}");
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        winresource::WindowsResource::new()
            .set_icon(ICON)
            .compile()
            .unwrap_or_else(|error| panic!("could not embed {ICON} in the exe: {error}"));
    }
}
