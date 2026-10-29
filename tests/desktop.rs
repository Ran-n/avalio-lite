// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.356749
//+ Revised: 	2026/10/29 01:00:47.089106
// -----------------------------------------------------------------------------

//! The real app in its real window: started through `examples/app_driver.rs` on scratch data,
//! clicked and typed into through Windows messages, and checked pixel by pixel.
//!
//! These tests open windows, so they are left out of the default run:
//!   cargo test --test desktop -- --ignored
//! Every screen is checked in light and dark, at the normal and the minimum window size, and in
//! Galician (the longer language) at the minimum size, against reviewed baselines in
//! `tests/desktop/baselines/`. After a deliberate change to the look:
//!   $env:AVALIO_UPDATE_BASELINES = "1"; cargo test --test desktop -- --ignored
//! then open every rewritten baseline, check nothing is clipped, wrapped or cut off, and commit.

mod common;
mod desktop_support;

use std::path::PathBuf;

use common::Fixture;
use desktop_support::pixels::{Image, Rect};
use desktop_support::{App, ThemeColours};
use serde_json::Value;

const NORMAL: (u32, u32) = (1280, 800);
/// The window's minimum size (`ui/app-window.slint`).
const MINIMUM: (u32, u32) = (1000, 600);
const PIN: &str = "2468";
/// What the setup tests type as the folders, so the screen doesn't depend on where the test runs.
const SHOWN_FOLDER: &str = r"D:\Marks";

/// Light and dark, at both sizes; `check` runs on each capture.
fn every_look(app: &mut App, screen: &str, check: impl FnMut(&mut App, &Image, bool, (u32, u32))) {
    every_look_after(app, screen, |_| {}, check);
}

/// Like `every_look`, with `prepare` run after each resize (a scroll position, say, which a new
/// window size changes).
fn every_look_after(
    app: &mut App,
    screen: &str,
    mut prepare: impl FnMut(&mut App),
    mut check: impl FnMut(&mut App, &Image, bool, (u32, u32)),
) {
    for dark in [false, true] {
        for size in [NORMAL, MINIMUM] {
            app.resize(size.0, size.1);
            prepare(app);
            app.theme(dark);
            let theme = if dark { "dark" } else { "light" };
            let image = app.snapshot(&format!("{screen}-{theme}-{}x{}", size.0, size.1));
            check(app, &image, dark, size);
        }
    }
}

/// The Galician screen at the minimum size: the longest labels in the least room.
fn galician_look(app: &mut App, screen: &str) -> Image {
    app.resize(MINIMUM.0, MINIMUM.1);
    app.theme(false);
    app.snapshot(&format!("{screen}-gl-{}x{}", MINIMUM.0, MINIMUM.1))
}

/// `control` lies wholly inside the window: nothing pushed off-screen or cut at the edge.
fn assert_on_screen(image: &Image, control: Rect, what: &str) {
    assert!(control.width > 0 && control.height > 0, "{what} has no size");
    assert!(
        control.right() <= image.width && control.bottom() <= image.height,
        "{what} at {control:?} runs past the {}x{} window",
        image.width,
        image.height
    );
}

fn located(app: &mut App, label: &str) -> Rect {
    let found = app.locate(label);
    assert!(!found.is_empty(), "{label:?} isn't on screen");
    found[0]
}

/// The settings point at `datavalio-lite.db` and `logs`, relative to the scratch folder the app
/// runs in, so the settings page shows the same paths on every run.
fn relative_paths(fixture: &Fixture) {
    let mut config = fixture.store().load().unwrap().unwrap();
    config.database_path = PathBuf::from("data").join(avalio_lite::config::DATABASE_FILE_NAME);
    config.log_dir = PathBuf::from("logs");
    fixture.store().save(&config).unwrap();
}

/// Settings with a database in `data` and logs in `logs`, and nothing else yet.
fn with_database() -> Fixture {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    relative_paths(&fixture);
    fixture
}

fn with_sample_class() -> Fixture {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    common::sample_class(state.connection().unwrap());
    state.shutdown();
    relative_paths(&fixture);
    fixture
}

fn config(fixture: &Fixture) -> PathBuf {
    fixture.path("config")
}

fn cells(state: &Value, row: usize) -> Vec<Value> {
    state["rows"][row]["cells"].as_array().unwrap().clone()
}

// ---------------------------------------------------------------------------------------------

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn first_run_setup() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    for language in ["en", "gl"] {
        let fixture = Fixture::new();
        let mut app = App::start(&config(&fixture), language);
        let (database, logs) = if language == "en" {
            ("Database folder", "Log folder")
        } else {
            ("Cartafol da base de datos", "Cartafol dos rexistros")
        };
        app.type_into(database, SHOWN_FOLDER);
        app.type_into(logs, SHOWN_FOLDER);
        if language == "en" {
            every_look(&mut app, "setup", |app, image, dark, _| {
                // the brand tile and the primary button are the accent colour, both whole
                let accent: Vec<Rect> = image
                    .blobs(colours.get("accent", dark), 6, 200)
                    .into_iter()
                    .filter(|blob| blob.height > 20)
                    .collect();
                assert_eq!(accent.len(), 2, "brand tile and Create button: {accent:?}");
                assert_on_screen(image, located(app, "Create database"), "Create database");
                assert_on_screen(image, located(app, "Open an existing database…"), "Open an existing database");
            });
            app.type_into(database, &fixture.path("data").display().to_string());
            app.type_into(logs, &fixture.path("logs").display().to_string());
            app.click("Create database");
            let state = app.state();
            assert_eq!(state["setup_required"], false, "{state}");
            assert!(fixture.database().is_file(), "a real click created the database");
        } else {
            let image = galician_look(&mut app, "setup");
            assert_on_screen(&image, located(&mut app, "Crear base de datos"), "Crear base de datos");
            assert_on_screen(&image, located(&mut app, "Abrir unha base de datos existente…"), "Abrir…");
        }
        app.quit();
    }
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_first_class() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_database();
    let mut app = App::start(&config(&fixture), "en");
    every_look(&mut app, "no-class", |app, image, dark, _| {
        let create = located(app, "Create class");
        assert_on_screen(image, create, "Create class");
        let accent = image.blobs(colours.get("accent", dark), 6, 200);
        assert!(accent.iter().any(|blob| blob.x.abs_diff(create.x) < 4), "Create class is the primary button");
    });
    app.type_into("Class name, e.g. 2º ESO A", "1A");
    app.click("Create class");
    let state = app.state();
    assert_eq!(state["class_name"], "1A", "{state}");
    assert_eq!(state["classes"], serde_json::json!(["1A"]));
    // a second class from the sidebar field, confirmed with the Enter key
    app.type_into("New class", "2B");
    app.press_enter();
    let state = app.state();
    assert_eq!(state["class_name"], "2B", "{state}");
    app.quit();

    let empty = with_database();
    let mut app = App::start(&config(&empty), "gl");
    let image = galician_look(&mut app, "no-class");
    assert_on_screen(&image, located(&mut app, "Crear clase"), "Crear clase");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_sheet() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    let state = app.state();
    // the final grade and its rounding are pills; a rounding can pass where the grade doesn't
    let pills: Vec<Value> = (0..5).flat_map(|row| cells(&state, row)[3..5].to_vec()).collect();
    let passing = pills.iter().filter(|cell| cell["tone"] == "pass").count();
    let failing = pills.iter().filter(|cell| cell["tone"] == "fail").count();
    assert!(passing > 0 && failing > 0, "the sample has both: {pills:?}");

    every_look(&mut app, "sheet", |app, image, dark, size| {
        assert_on_screen(image, located(app, "+ Add column"), "+ Add column");
        assert_on_screen(image, located(app, "More actions"), "More actions");
        if size == NORMAL {
            // every final grade and its rounding is a pill: pass pills green, fail pills rose
            let green = image.blobs(colours.get("pass-soft", dark), 4, 300);
            let rose = image.blobs(colours.get("fail-soft", dark), 4, 300);
            assert_eq!(green.len(), passing, "pass pills: {green:?}");
            assert_eq!(rose.len(), failing, "fail pills: {rose:?}");
            for pill in green.iter().chain(&rose) {
                assert_on_screen(image, *pill, "a grade pill");
            }
            // pills of one row line up, give or take a pixel or two
            let mut tops: Vec<u32> = green.iter().chain(&rose).map(|pill| pill.y).collect();
            tops.sort_unstable();
            let rows = 1 + tops.windows(2).filter(|pair| pair[1] - pair[0] > 4).count();
            assert_eq!(rows, 5, "pills sit on the five pupil rows: {tops:?}");
        }
    });

    // a real mark, typed into the real cell: Daniel's exam goes from 4 to 9 and he passes
    app.type_into("Daniel Souto, Exam 1", "9");
    let after = app.state();
    assert_eq!(cells(&after, 3)[0]["text"], "9");
    assert_eq!(cells(&after, 3)[3]["tone"], "pass", "{after}");
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    let image = galician_look(&mut app, "sheet");
    assert_on_screen(&image, located(&mut app, "+ Engadir columna"), "+ Engadir columna");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_column_panel_and_menus() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Exam 1");
    assert_eq!(app.state()["panel_index"], 0);
    every_look(&mut app, "column-panel", |app, image, _, _| {
        for control in ["Close", "‹ Move left", "Move right ›", "Delete", "Bonus test"] {
            assert_on_screen(image, located(app, control), control);
        }
    });
    app.click("Move right ›");
    let state = app.state();
    assert_eq!(state["columns"][1], "Exam 1", "a real click moved the column: {state}");
    app.click("Close");

    app.click("+ Add column");
    every_look(&mut app, "add-column-menu", |app, image, _, _| {
        for option in ["Test", "Info", "Final grade", "Average", "Rounded"] {
            let found = app.locate_in_menu(option);
            assert_eq!(found.len(), 1, "{option}");
            assert_on_screen(image, found[0], option);
        }
    });
    app.click_in_menu("Info");
    assert_eq!(app.state()["columns"].as_array().unwrap().len(), 7, "a real click added a column");
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    app.click("Exam 1");
    let image = galician_look(&mut app, "column-panel");
    for english in ["Close", "‹ Move left", "Move right ›", "Delete"] {
        let label = common::galician(english);
        assert_on_screen(&image, located(&mut app, &label), &label);
    }
    app.click(&common::galician("Close"));
    app.click(&common::galician("+ Add column"));
    let image = galician_look(&mut app, "add-column-menu");
    for english in ["Test", "Info", "Final grade", "Average", "Rounded"] {
        let label = common::galician(english);
        let found = app.locate_in_menu(&label);
        assert_eq!(found.len(), 1, "{label}");
        assert_on_screen(&image, found[0], &label);
    }
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn settings() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    every_look(&mut app, "settings", |app, image, _, size| {
        for control in ["Light", "Dark", "Galego", "English"] {
            assert_on_screen(image, located(app, control), control);
        }
        // the page scrolls; in the smallest window the PIN card starts below the fold
        if size == NORMAL {
            assert_on_screen(image, located(app, "Encrypt with this PIN"), "Encrypt with this PIN");
        }
    });
    every_look_after(
        &mut app,
        "settings-folders",
        |app| app.scroll("Galego", -40),
        |app, image, _, _| {
            assert_on_screen(image, located(app, "Open another database…"), "Open another database");
            for (index, what) in ["the database's Move", "the logs' Move"].into_iter().enumerate() {
                let found = app.locate("Move…");
                assert_on_screen(image, found[index], what);
            }
        },
    );
    // real clicks through a real flow: move the logs to a folder the "picker" answers with
    // relative to the scratch folder the app runs in, so later screens show a stable path
    let destination = PathBuf::from("moved-logs");
    app.answer_dialogs(&[&destination], &[], &[]);
    app.click_nth("Move…", 1);
    let state = app.state();
    assert!(state["status"].as_str().unwrap().starts_with("Logs moved to"), "{state}");
    app.quit();
    assert!(fixture.path("moved-logs").join(avalio_lite::config::LOG_FILE_NAME).is_file());

    let mut app = App::start(&config(&fixture), "gl");
    app.click("Axustes");
    let image = galician_look(&mut app, "settings");
    for control in ["Claro", "Escuro", "Galego", "English"] {
        assert_on_screen(&image, located(&mut app, control), control);
    }
    app.scroll("Galego", -40);
    let image = galician_look(&mut app, "settings-folders");
    assert_on_screen(&image, located(&mut app, "Abrir outra base de datos…"), "Abrir outra base de datos…");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_lock_screen() {
    let _turn = desktop_support::turn();
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    common::sample_class(state.connection().unwrap());
    state.set_pin(PIN, PIN).unwrap();
    state.shutdown();
    relative_paths(&fixture);

    let mut app = App::start(&config(&fixture), "en");
    assert_eq!(app.state()["locked"], true);
    every_look(&mut app, "lock", |app, image, _, _| {
        for key in ["1", "5", "9", "0", "Letters", "Erase", "Unlock", "Forgot the PIN?"] {
            assert_on_screen(image, located(app, key), key);
        }
    });
    app.click("Letters");
    every_look(&mut app, "lock-letters", |app, image, _, _| {
        for key in ["q", "p", "a", "ñ", "z", "m", "Capital letters", "Erase", "Numbers", "Space", "Unlock"] {
            assert_on_screen(image, located(app, key), key);
        }
    });
    app.click("Symbols");
    every_look(&mut app, "lock-symbols", |app, image, _, _| {
        for key in ["!", "-", "_", "'", "\"", "|", "More symbols", "Erase", "Numbers", "Letters", "Space"] {
            assert_on_screen(image, located(app, key), key);
        }
    });
    app.click("More symbols");
    every_look(&mut app, "lock-symbols-2", |app, image, _, _| {
        for key in ["~", "€", "¿", "¥", "«", "µ", "More symbols"] {
            assert_on_screen(image, located(app, key), key);
        }
    });
    app.click("Numbers");
    app.click("Forgot the PIN?");
    assert_eq!(app.state()["setup_required"], true);
    // the suggested folders are the test's scratch folder; a fixed one keeps the baselines stable
    app.type_into("Database folder", SHOWN_FOLDER);
    app.type_into("Log folder", SHOWN_FOLDER);
    every_look(&mut app, "left-locked", |app, image, _, _| {
        for control in ["Back to the lock screen", "Open an existing database…", "Create database"] {
            assert_on_screen(image, located(app, control), control);
        }
    });
    app.click("Back to the lock screen");
    let state = app.state();
    assert_eq!(state["setup_required"], false, "{state}");
    assert_eq!(state["locked"], true, "back on the lock screen: {state}");
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

    let mut app = App::start(&config(&fixture), "gl");
    let image = galician_look(&mut app, "lock");
    assert_on_screen(&image, located(&mut app, "Desbloquear"), "Desbloquear");
    assert_on_screen(&image, located(&mut app, "Esqueciches o PIN?"), "Esqueciches o PIN?");
    app.click("Letras");
    let image = galician_look(&mut app, "lock-letters");
    for key in ["Maiúsculas", "Borrar", "Números", "Símbolos", "Espazo", "Esqueciches o PIN?"] {
        assert_on_screen(&image, located(&mut app, key), key);
    }
    app.click("Símbolos");
    let image = galician_look(&mut app, "lock-symbols");
    for key in ["Máis símbolos", "Letras", "Números", "Espazo"] {
        assert_on_screen(&image, located(&mut app, key), key);
    }
    app.click("Esqueciches o PIN?");
    app.type_into("Cartafol da base de datos", SHOWN_FOLDER);
    app.type_into("Cartafol dos rexistros", SHOWN_FOLDER);
    let image = galician_look(&mut app, "left-locked");
    for control in ["Volver á pantalla de bloqueo", "Abrir unha base de datos existente…", "Crear base de datos"] {
        assert_on_screen(&image, located(&mut app, control), control);
    }
    app.quit();
}

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
        let button = located(&mut app, if dark { "Light mode" } else { "Dark mode" });
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
