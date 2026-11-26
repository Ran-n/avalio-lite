// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.993460
//+ Revised: 	2026/11/24 17:54:09.162029
// -----------------------------------------------------------------------------

//! The smallest real-world screen (1366×768 at 125 %, taskbar out: 1092×582 logical, where the app
//! opens maximised with its minimum lowered): every screen's controls stay reachable, and the sheet,
//! its column panel, Settings and the lock screen each fit.

use crate::common::Fixture;
use crate::desktop_support::{self, App};

use super::fixtures::{PANEL_CONTROLS, PIN, config, relative_paths, with_sample_class};
use super::screens::assert_on_screen;

const SMALL: (u32, u32) = (1092, 582);

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_sheet_fits_a_small_screen() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.minimum(SMALL.0, SMALL.1);
    app.resize(SMALL.0, SMALL.1);
    app.theme(false);
    let image = app.snapshot(&format!("sheet-light-{}x{}", SMALL.0, SMALL.1));
    assert_eq!(
        (image.width, image.height),
        (app.px(SMALL.0 as f64), app.px(SMALL.1 as f64)),
        "the window took the size"
    );
    for control in ["+ Add column", "More actions", "Settings"] {
        assert_on_screen(&image, app.located(control), control);
    }
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_column_panel_fits_a_small_screen() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.minimum(SMALL.0, SMALL.1);
    app.resize(SMALL.0, SMALL.1);
    app.theme(false);
    app.click("Exam 1");
    let image = app.capture();
    for control in PANEL_CONTROLS.iter().chain(&["Bonus test"]) {
        assert_on_screen(&image, app.located(control), control);
    }
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_settings_page_fits_a_small_screen() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.minimum(SMALL.0, SMALL.1);
    app.resize(SMALL.0, SMALL.1);
    app.theme(false);
    app.click("Settings");
    let image = app.snapshot(&format!("settings-light-{}x{}", SMALL.0, SMALL.1));
    for control in ["Light", "Dark", "Galego", "English"] {
        assert_on_screen(&image, app.located(control), control);
    }
    app.scroll_main(-10);
    let image = app.capture();
    assert_on_screen(&image, app.located("Encrypt with this PIN"), "Encrypt with this PIN");
    app.show_folder_cards("Move…");
    let image = app.capture();
    assert_on_screen(&image, app.located("Open another database…"), "Open another database…");
    for (index, what) in ["the database's Move", "the logs' Move"].into_iter().enumerate() {
        assert_on_screen(&image, app.locate("Move…")[index], what);
    }
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_lock_screen_fits_a_small_screen() {
    let _turn = desktop_support::turn();
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin(PIN, PIN).unwrap();
    state.shutdown();
    relative_paths(&fixture);
    let mut app = App::start(&config(&fixture), "en");
    app.minimum(SMALL.0, SMALL.1);
    app.resize(SMALL.0, SMALL.1);
    let image = app.capture();
    for key in ["1", "5", "9", "0", "Letters", "Erase", "Unlock", "Forgot the PIN?"] {
        assert_on_screen(&image, app.located(key), key);
    }
    app.quit();
}
