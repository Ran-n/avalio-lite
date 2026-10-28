// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:33.763628
//+ Revised: 	2026/10/28 15:53:07.995855
// -----------------------------------------------------------------------------

use slint_build::{CompilerConfiguration, DefaultTranslationContext};

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
}
