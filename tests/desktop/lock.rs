// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.787489
//+ Revised: 	2026/11/25 11:43:12.228929
// -----------------------------------------------------------------------------

//! The lock screen: its keypads (numbers, letters, symbols) in English and Galician, the Forgot the
//! PIN screen and going back from it, and unlocking with the PIN and locking again.

use crate::desktop_support::{self, App};

use super::fixtures::{PIN, SHOWN_FOLDER, config, with_locked_sample};
use super::screens::{assert_on_screen, every_look, galician_look, galician_window};

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_lock_keypads_look_right() {
    let _turn = desktop_support::turn();
    let fixture = with_locked_sample();
    let mut app = App::start(&config(&fixture), "en");
    assert_eq!(app.state()["locked"], true);
    every_look(&mut app, "lock", |app, image, _, _| {
        for key in ["1", "5", "9", "0", "Letters", "Erase", "Unlock", "Forgot the PIN?"] {
            assert_on_screen(image, app.located(key), key);
        }
    });
    app.click("Letters");
    every_look(&mut app, "lock-letters", |app, image, _, _| {
        for key in ["q", "p", "a", "ñ", "z", "m", "Capital letters", "Erase", "Numbers", "Space", "Unlock"] {
            assert_on_screen(image, app.located(key), key);
        }
    });
    app.click("Symbols");
    every_look(&mut app, "lock-symbols", |app, image, _, _| {
        for key in ["!", "-", "_", "'", "\"", "|", "More symbols", "Erase", "Numbers", "Letters", "Space"] {
            assert_on_screen(image, app.located(key), key);
        }
    });
    app.click("More symbols");
    every_look(&mut app, "lock-symbols-2", |app, image, _, _| {
        for key in ["~", "€", "¿", "¥", "«", "µ", "More symbols"] {
            assert_on_screen(image, app.located(key), key);
        }
    });
    app.click("Numbers");
    assert_eq!(app.locate("5").len(), 1, "a real click on Numbers brought the number keypad back");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_forgot_the_pin_screen_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_locked_sample();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Forgot the PIN?");
    assert_eq!(app.state()["setup_required"], true);
    // the suggested folders are the test's scratch folder; a fixed one keeps the baselines stable
    app.type_into("Database folder", SHOWN_FOLDER);
    app.type_into("Log folder", SHOWN_FOLDER);
    every_look(&mut app, "left-locked", |app, image, _, _| {
        for control in ["Back to the lock screen", "Open an existing database…", "Create database"] {
            assert_on_screen(image, app.located(control), control);
        }
    });
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn back_to_the_lock_screen_keeps_it_locked() {
    let _turn = desktop_support::turn();
    let fixture = with_locked_sample();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Forgot the PIN?");
    app.click("Back to the lock screen");
    let state = app.state();
    assert_eq!(state["setup_required"], false, "{state}");
    assert_eq!(state["locked"], true, "back on the lock screen: {state}");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_pin_unlocks_and_the_sidebar_button_locks_again() {
    let _turn = desktop_support::turn();
    let fixture = with_locked_sample();
    let mut app = App::start(&config(&fixture), "en");
    for key in ["1", "Erase"].into_iter().chain(PIN.split("").filter(|digit| !digit.is_empty())) {
        app.click(key);
    }
    app.click("Unlock");
    let state = app.state();
    assert_eq!(state["locked"], false, "real clicks on the keypad unlocked it: {state}");
    assert_eq!(state["database_open"], true);
    app.click("Lock (Ctrl+L)");
    assert_eq!(app.state()["locked"], true, "and the sidebar button locks it again");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_lock_keypads_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_locked_sample();
    let mut app = App::start(&config(&fixture), "gl");
    let image = galician_look(&mut app, "lock");
    assert_on_screen(&image, app.located("Desbloquear"), "Desbloquear");
    assert_on_screen(&image, app.located("Esqueciches o PIN?"), "Esqueciches o PIN?");
    app.click("Letras");
    let image = galician_look(&mut app, "lock-letters");
    for key in ["Maiúsculas", "Borrar", "Números", "Símbolos", "Espazo", "Esqueciches o PIN?"] {
        assert_on_screen(&image, app.located(key), key);
    }
    app.click("Símbolos");
    let image = galician_look(&mut app, "lock-symbols");
    for key in ["Máis símbolos", "Letras", "Números", "Espazo"] {
        assert_on_screen(&image, app.located(key), key);
    }
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn forgot_the_pin_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_locked_sample();
    let mut app = App::start(&config(&fixture), "gl");
    // the keypad is reached as a user reaches it: Letters, then Symbols, then the Forgot link, each
    // at the size and look its Galician look is captured at
    galician_window(&mut app);
    app.click("Letras");
    galician_window(&mut app);
    app.click("Símbolos");
    galician_window(&mut app);
    app.click("Esqueciches o PIN?");
    app.type_into("Cartafol da base de datos", SHOWN_FOLDER);
    app.type_into("Cartafol dos rexistros", SHOWN_FOLDER);
    let image = galician_look(&mut app, "left-locked");
    for control in ["Volver á pantalla de bloqueo", "Abrir unha base de datos existente…", "Crear base de datos"] {
        assert_on_screen(&image, app.located(control), control);
    }
    app.quit();
}
