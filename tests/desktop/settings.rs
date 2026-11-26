// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.369463
//+ Revised: 	2026/11/24 17:54:08.762361
// -----------------------------------------------------------------------------

//! The Settings page: its looks in English and Galician (the top, the shortcuts, a row recording its
//! keys, and the folder cards), the folder cards' scroll, and moving the logs to another folder.
//! The Galician looks follow the logs being moved, so their folder cards show the new place.

use std::path::PathBuf;

use crate::common::Fixture;
use crate::desktop_support::{self, App};

use super::fixtures::{config, with_sample_class};
use super::screens::{NORMAL, assert_on_screen, every_look, every_look_after, galician_look, galician_window};

/// Moves the logs to `moved-logs`, relative to the scratch folder the app runs in, so later screens
/// show a stable path.
fn move_logs(app: &mut App) {
    app.show_folder_cards("Move…");
    let destination = PathBuf::from("moved-logs");
    app.answer_dialogs(&[&destination], &[], &[]);
    app.click_nth("Move…", 1);
}

/// The sample class, with its logs moved to `moved-logs` as a user would have: the Galician looks
/// are captured after that.
fn with_logs_moved() -> Fixture {
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    move_logs(&mut app);
    app.quit();
    fixture
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_settings_page_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    every_look(&mut app, "settings", |app, image, _, size| {
        for control in ["Light", "Dark", "Galego", "English"] {
            assert_on_screen(image, app.located(control), control);
        }
        // the page scrolls; in the smallest window the shortcuts card ends below the fold
        if size == NORMAL {
            for row in ["Lock", "Switch between light and dark", "Collapse or expand the sidebar"] {
                assert_on_screen(image, app.located(row), row);
            }
        }
    });
    every_look_after(
        &mut app,
        "settings-pin",
        |app| {
            app.scroll_main(40);
            app.scroll_main(-10);
        },
        |app, image, _, _| {
            assert_on_screen(image, app.located("Encrypt with this PIN"), "Encrypt with this PIN");
        },
    );
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_folder_cards_look_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    every_look_after(
        &mut app,
        "settings-folders",
        |app| app.show_folder_cards("Move…"),
        |app, image, _, _| {
            assert_on_screen(image, app.located("Open another database…"), "Open another database");
            for (index, what) in ["the database's Move", "the logs' Move"].into_iter().enumerate() {
                let found = app.locate("Move…");
                assert_on_screen(image, found[index], what);
            }
        },
    );
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn moving_the_logs_to_another_folder() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    move_logs(&mut app);
    let state = app.state();
    assert!(state["status"].as_str().unwrap().starts_with("Logs moved to"), "{state}");
    app.quit();
    assert!(fixture.path("moved-logs").join(avalio_lite::platform::config::LOG_FILE_NAME).is_file());
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_settings_page_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_logs_moved();
    let mut app = App::start(&config(&fixture), "gl");
    app.click("Axustes");
    let image = galician_look(&mut app, "settings");
    for control in ["Claro", "Escuro", "Galego", "English"] {
        assert_on_screen(&image, app.located(control), control);
    }
    app.scroll("Galego", -4);
    let image = galician_look(&mut app, "settings-shortcuts");
    for row in ["Bloquear", "Cambiar entre claro e escuro", "Amosar ou agochar pesos e escalas", "Clase anterior"] {
        assert_on_screen(&image, app.located(row), row);
    }
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_row_recording_its_keys_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_logs_moved();
    let mut app = App::start(&config(&fixture), "gl");
    app.click("Axustes");
    // the window the Galician looks leave is the one the row is clicked at, and it is scrolled first
    galician_window(&mut app);
    app.scroll("Galego", -4);
    app.click_nth("Cambiar", 1);
    let image = galician_look(&mut app, "settings-recording");
    assert_on_screen(&image, app.located("Cancelar"), "Cancelar");
    app.click("Cancelar");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_folder_cards_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_logs_moved();
    let mut app = App::start(&config(&fixture), "gl");
    app.click("Axustes");
    // the window the Galician looks leave, before the cards are scrolled into view
    galician_window(&mut app);
    app.show_folder_cards("Mover…");
    let image = galician_look(&mut app, "settings-folders");
    assert_on_screen(&image, app.located("Abrir outra base de datos…"), "Abrir outra base de datos…");
    app.quit();
}
