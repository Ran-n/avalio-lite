// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:01.958478
//+ Revised: 	2026/11/24 12:35:14.378455
// -----------------------------------------------------------------------------

//! The About card at the foot of Settings: its look in English and Galician, and the source-code link
//! asking Windows to open the repository.

use serde_json::json;

use crate::desktop_support::{self, App};

use super::fixtures::{config, with_sample_class};
use super::screens::{MINIMUM, assert_on_screen, every_look_after, galician_look};

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_about_section_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    every_look_after(&mut app, "settings-about", App::show_about, |app, image, _, _| {
        for control in ["Open the Breren website", "Read the licence", "Report a problem", "Write to support"] {
            assert_on_screen(image, app.located(control), control);
        }
        assert_on_screen(image, app.located("Breren"), "the Breren logo");
        let (logo, website) = (app.located("Breren"), app.located("Open the Breren website"));
        let (logo_middle, website_middle) = (logo.y + logo.height / 2, website.y + website.height / 2);
        assert!(
            logo_middle.abs_diff(website_middle) <= app.px(1.0),
            "the website link is level with the logo: {website:?} beside {logo:?}"
        );
    });
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_source_code_link_asks_windows_to_open_the_repository() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    app.show_about();
    // a real click asks Windows to open the page (the driver only notes it)
    app.click("Source code");
    let state = app.state();
    assert_eq!(state["links"], json!(["https://github.com/Ran-n/avalio-lite"]), "{state}");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_about_section_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "gl");
    app.click(&crate::common::galician("Settings"));
    app.resize(MINIMUM.0, MINIMUM.1);
    app.show_about();
    let image = galician_look(&mut app, "settings-about");
    for control in ["Read the licence", "Report a problem", "Write to support"] {
        let words = crate::common::galician(control);
        assert_on_screen(&image, app.located(&words), &words);
    }
    app.quit();
}
