// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.614966
//+ Revised: 	2026/11/24 17:54:07.922925
// -----------------------------------------------------------------------------

//! The keyboard moves the focus through a screen's controls, and Enter presses the focused one: the
//! focus ring is drawn in the accent colour, at each theme.

use crate::desktop_support::{self, App, ThemeColours};

use super::fixtures::{config, with_sample_class};

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn keyboard_focus() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    for dark in [false, true] {
        app.theme(dark);
        app.click("New class");
        app.press_tab();
        let button = app.located(if dark { "Light mode" } else { "Dark mode" });
        let image = app.capture();
        // the focused button is outlined in the accent colour; inside the outline it is unchanged
        let edge = image.pixel(button.x + 1, button.y + button.height / 2);
        assert!(edge.near(colours.get("accent", dark), 24), "focus ring at the button's edge: {edge:?}");
        app.press_enter();
        assert_eq!(app.state()["dark"], !dark, "Enter pressed the focused theme button");
        app.theme(dark);
    }
    app.quit();
}
