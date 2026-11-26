// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:05.491051
//+ Revised: 	2026/11/24 17:54:10.697954
// -----------------------------------------------------------------------------

//! The theme's colour tokens, read from `ui/theme.slint` itself, so a test checks the colour a
//! screen should show by the token's name, in light or dark, and never by a copied hex value.

use std::collections::HashMap;

use super::manifest_dir;
use super::pixels::Rgb;

/// The colour tokens of `ui/theme.slint`, light and dark, read from the file itself.
pub struct ThemeColours {
    tokens: HashMap<String, (Rgb, Rgb)>,
}

impl ThemeColours {
    pub fn load() -> Self {
        let source = std::fs::read_to_string(manifest_dir().join("ui").join("theme.slint")).unwrap();
        let mut tokens = HashMap::new();
        for line in source.lines() {
            let Some(rest) = line.trim().strip_prefix("out property <color> ") else { continue };
            let Some((name, value)) = rest.split_once(": dark ? ") else { continue };
            let Some((dark, light)) = value.trim_end_matches(';').split_once(" : ") else { continue };
            // only opaque colours; shadows have an alpha and never show as themselves
            if dark.len() == 7 && light.len() == 7 {
                tokens.insert(name.to_owned(), (Rgb::hex(dark), Rgb::hex(light)));
            }
        }
        assert!(tokens.len() > 15, "theme.slint's colours weren't found");
        Self { tokens }
    }

    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "the look under test, light or dark, which every desktop screen is checked in"
    )]
    pub fn get(&self, name: &str, dark: bool) -> Rgb {
        let (dark_colour, light_colour) = self.tokens[name];
        if dark { dark_colour } else { light_colour }
    }
}
