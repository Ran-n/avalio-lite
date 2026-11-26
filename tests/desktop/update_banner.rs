// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:04.216880
//+ Revised: 	2026/11/24 17:54:09.394437
// -----------------------------------------------------------------------------

//! The update banner and the Settings updates card: the card's look in English and Galician, a check
//! that offers a version, the offer's bar in the sidebar (looks, and Hide), a check that hangs, and a
//! build from source with no update controls. The fake forge serves the releases.

use std::time::Duration;

use serde_json::json;

use crate::common::Fixture;
use crate::desktop_support::pixels::Image;
use crate::desktop_support::{self, App};

use super::fixtures::{config, with_sample_class};
use super::screens::{assert_on_screen, every_look_after, galician_look, galician_window};

/// The English half of the update flow, up to the banner's Hide: the check, the class clicked and the
/// banner hidden. The Galician updates card is captured after it, so the setup repeats it.
fn english_update_flow(fixture: &Fixture) {
    let mut app = App::start(&config(fixture), "en");
    app.click("Settings");
    app.send(json!({"cmd": "release", "version": "9.0.0"}));
    app.scroll_into_view("Check for updates now");
    app.click("Check for updates now");
    app.wait_for("the check", |state| state["banner"] == "offer");
    app.click("2º ESO A");
    app.click("Hide");
    app.quit();
}

/// The banner's two buttons, Update and restart and Skip this version, are on screen.
fn check_bar(app: &mut App, image: &Image, install: &str, skip: &str) {
    assert_on_screen(image, app.located(install), "Update and restart");
    assert_on_screen(image, app.located(skip), "Skip this version");
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_settings_updates_card_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    // before any check: "Not checked for updates yet.", so the picture is the same every run
    every_look_after(
        &mut app,
        "settings-updates",
        |app| app.scroll_into_view("Check for updates now"),
        |app, image, _, _| {
            assert_on_screen(image, app.located("Check for updates now"), "Check now");
            assert_on_screen(image, app.located_text("Not checked for updates yet."), "the status line");
        },
    );
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_settings_updates_card_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    english_update_flow(&fixture);
    let mut app = App::start(&config(&fixture), "gl");
    app.click(&crate::common::galician("Settings"));
    galician_window(&mut app);
    app.scroll_into_view(&crate::common::galician("Check for updates now"));
    let image = galician_look(&mut app, "settings-updates");
    assert_on_screen(&image, app.located(&crate::common::galician("Check for updates now")), "Check now");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn checking_for_an_update_offers_it() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    // a fixed version, so the banner's text is the same on every run
    app.send(json!({"cmd": "release", "version": "9.0.0"}));
    app.scroll_into_view("Check for updates now");
    app.click("Check for updates now");
    app.wait_for("the check", |state| state["banner"] == "offer");
    assert!(app.state()["update"].as_str().unwrap().contains("9.0.0"), "a real click checked");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_update_bar_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    app.send(json!({"cmd": "release", "version": "9.0.0"}));
    app.scroll_into_view("Check for updates now");
    app.click("Check for updates now");
    app.wait_for("the check", |state| state["banner"] == "offer");
    // the banner is the window's, not the sheet's: it shows on Settings, where the check was made
    let image = app.capture();
    check_bar(&mut app, &image, "Update and restart", "Skip this version");
    app.click("2º ESO A");
    every_look_after(
        &mut app,
        "update-bar",
        |_| {},
        |app, image, _, _| {
            check_bar(app, image, "Update and restart", "Skip this version");
        },
    );
    app.click("Hide");
    assert_eq!(app.state()["banner"], "hidden", "a real click hid the banner");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_update_bar_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    english_update_flow(&fixture);
    let (install, skip) = (crate::common::galician("Update and restart"), crate::common::galician("Skip this version"));
    let mut app = App::start(&config(&fixture), "gl");
    app.click(&crate::common::galician("Settings"));
    galician_window(&mut app);
    app.scroll_into_view(&crate::common::galician("Check for updates now"));
    app.send(json!({"cmd": "release", "version": "9.0.0"}));
    app.scroll_into_view(&crate::common::galician("Check for updates now"));
    app.click(&crate::common::galician("Check for updates now"));
    app.wait_for("the check", |state| state["banner"] == "offer");
    assert!(app.state()["update"].as_str().unwrap().contains("9.0.0"), "a real click checked");
    let image = app.capture();
    check_bar(&mut app, &image, &install, &skip);
    app.click("2º ESO A");
    let image = galician_look(&mut app, "update-bar");
    check_bar(&mut app, &image, &install, &skip);
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn quitting_during_a_hanging_check_closes_at_once() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.send(json!({"cmd": "release", "version": "9.0.0", "hang": true}));
    app.click("Settings");
    app.scroll_into_view("Check for updates now");
    app.click("Check for updates now");
    assert!(!app.locate("Checking…").is_empty(), "the check is under way");
    let took = app.quit_timed();
    assert!(took < Duration::from_secs(5), "quitting took {took:?}: the check is abandoned, not waited for");
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_build_from_source_shows_no_update_controls() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start_from_source(&config(&fixture), "en");
    app.click("Settings");
    let source = "This copy was built from source. Update it with git pull and a new build.";
    app.scroll_main(60);
    for _ in 0..80 {
        if !app.locate_text(source).is_empty() {
            break;
        }
        app.scroll_main(-1);
    }
    app.scroll_main(-1);
    let image = app.capture();
    assert_on_screen(&image, app.located_text(source), "the source build's line");
    for control in ["Check for updates now", "Update and restart"] {
        assert!(app.locate(control).is_empty(), "{control} shows on a build from source");
    }
    app.quit();
}
