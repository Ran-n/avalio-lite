// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.704930
//+ Revised: 	2026/11/24 17:54:08.025709
// -----------------------------------------------------------------------------

//! The licence: the licence screen in English and Galician, a key the fake Clavren refuses as in use
//! elsewhere, activating a key, the Settings licence card, and the warning to connect to the internet.

use serde_json::json;

use crate::desktop_support::{self, App};

use super::fixtures::{config, with_database, with_sample_class};
use super::screens::{MINIMUM, NORMAL, assert_on_screen, every_look, every_look_after, galician_look, galician_window};

/// A key the fake Clavren says is already activated on another computer.
const KEY: &str = "BRN-79YN-44XQ-W025-377Z";

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_licence_screen_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_database();
    let mut app = App::start_unlicensed(&config(&fixture), "en");
    assert_eq!(app.state()["licensed"], false);
    every_look(&mut app, "licence", |app, image, _, _| {
        for control in ["Licence key", "Activate", "Back up my data"] {
            assert_on_screen(image, app.located(control), control);
        }
    });
    app.quit();

    let mut app = App::start_unlicensed(&config(&fixture), "gl");
    let image = galician_look(&mut app, "licence");
    for control in ["Clave de licenza", "Activar", "Facer copia dos meus datos"] {
        assert_on_screen(&image, app.located(control), control);
    }
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_key_in_use_elsewhere_is_refused_on_screen() {
    let _turn = desktop_support::turn();
    let fixture = with_database();
    let mut app = App::start_unlicensed(&config(&fixture), "en");
    // the longest message, in the least room
    app.send(json!({ "cmd": "licence", "refuse": "already_activated", "status": 409 }));
    app.type_into("Licence key", KEY);
    app.click("Activate");
    app.resize(MINIMUM.0, MINIMUM.1);
    app.theme(false);
    let image = app.snapshot("licence-in-use-elsewhere-light-1000x600");
    assert_on_screen(&image, app.located("Back up my data"), "Back up my data");
    assert_eq!(app.state()["licensed"], false);
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_key_in_use_elsewhere_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_database();
    let mut app = App::start_unlicensed(&config(&fixture), "gl");
    galician_window(&mut app);
    app.send(json!({ "cmd": "licence", "refuse": "already_activated", "status": 409 }));
    app.type_into("Clave de licenza", KEY);
    app.click("Activar");
    let image = galician_look(&mut app, "licence-in-use-elsewhere");
    assert_on_screen(&image, app.located("Facer copia dos meus datos"), "Facer copia dos meus datos");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn activating_a_key_licenses_the_app() {
    let _turn = desktop_support::turn();
    let fixture = with_database();
    let mut app = App::start_unlicensed(&config(&fixture), "en");
    app.type_into("Licence key", KEY);
    app.click("Activate");
    let state = app.state();
    assert_eq!(state["licensed"], true, "real clicks activated the key: {state}");
    assert!(!app.locate("Settings").is_empty(), "the sheet shows");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_licence_card_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    every_look_after(
        &mut app,
        "settings-licence",
        |app| app.scroll_into_view("Check for updates now"),
        |app, image, _, _| {
            assert_on_screen(image, app.located("Check now"), "Check now");
            assert_on_screen(image, app.located_text("Licensed"), "Licensed");
        },
    );
    // the Galician card is captured after the English check, as a user would have left it
    app.click("Check now");
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    app.click("Axustes");
    galician_window(&mut app);
    app.scroll_into_view("Buscar actualizacións agora");
    let image = galician_look(&mut app, "settings-licence");
    assert_on_screen(&image, app.located("Comprobar agora"), "Comprobar agora");
    assert_on_screen(&image, app.located_text("Con licenza"), "Con licenza");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn checking_the_licence_confirms_it() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    app.scroll_into_view("Check for updates now");
    app.click("Check now");
    let state = app.state();
    assert_eq!(state["status"], "Licence confirmed.", "a real click checked the licence: {state}");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_offline_warning_fits_at_both_sizes() {
    let _turn = desktop_support::turn();
    // the warning names a day counted from today, so it is checked for room, not against a baseline
    let fixture = with_sample_class();
    for language in ["en", "gl"] {
        let mut app = App::start_offline_soon(&config(&fixture), language);
        let warning = app.state()["status"].as_str().unwrap().to_owned();
        assert!(warning.contains("internet"), "{warning}");
        for dark in [false, true] {
            for size in [NORMAL, MINIMUM] {
                app.resize(size.0, size.1);
                app.theme(dark);
                let image = app.capture();
                assert_on_screen(&image, app.located_text(&warning), "the offline warning");
            }
        }
        app.quit();
    }
}
