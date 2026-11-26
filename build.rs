// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/01 20:52:43.232212
//+ Revised: 	2026/11/24 14:30:29.902524
// -----------------------------------------------------------------------------

//! Compiles the Slint UI with its bundled translations, gives the Windows exe its icon, and keeps
//! OpenSSL's missing debug file out of the linker's output.

use slint_build::{CompilerConfiguration, DefaultTranslationContext};

const ICON: &str = "assets/logo.ico";

/// The linker's "PDB was not found; linking object as if no debug info".
const MISSING_PDB_WARNING: &str = "4099";

fn main() {
    compile_ui();
    embed_icon();
    ignore_missing_openssl_pdb();
}

fn compile_ui() {
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
}

/// The exe's own icon, shown by Explorer, the Start menu and the uninstall entry. A build without
/// it is broken, so a failure stops the build.
fn embed_icon() {
    println!("cargo:rerun-if-changed={ICON}");
    if target_is("CARGO_CFG_TARGET_OS", "windows") {
        winresource::WindowsResource::new()
            .set_icon(ICON)
            .compile()
            .unwrap_or_else(|error| panic!("could not embed {ICON} in the exe: {error}"));
    }
}

/// OpenSSL's MSVC build always writes its debug info to a separate `ossl_static.pdb`, which Rust's
/// libraries don't carry, so a debug build would warn once for each of its hundreds of objects. Only
/// stepping into OpenSSL's C code would need that file. Every other linker warning still shows.
fn ignore_missing_openssl_pdb() {
    if target_is("CARGO_CFG_TARGET_ENV", "msvc") {
        println!("cargo:rustc-link-arg=/IGNORE:{MISSING_PDB_WARNING}");
    }
}

fn target_is(variable: &str, expected: &str) -> bool {
    std::env::var(variable).is_ok_and(|value| value == expected)
}
