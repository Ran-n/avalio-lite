// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.356749
//+ Revised: 	2026/11/10 07:42:00.919316
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

use std::path::{Path, PathBuf};

use common::Fixture;
use desktop_support::pixels::{Image, Rect};
use desktop_support::{App, ThemeColours};
use serde_json::{Value, json};

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

/// Scrolls the main area from its top, a notch at a time, until the control labelled `label` is
/// wholly in the window: the same notches every run, so the picture is the same.
fn scroll_into_view(app: &mut App, label: &str) {
    app.scroll_main(60);
    for _ in 0..60 {
        let height = app.capture().height;
        if app.locate(label).first().is_some_and(|found| found.bottom() + app.px(16.0) <= height) {
            return;
        }
        app.scroll_main(-1);
    }
    panic!("{label:?} never came into view");
}

fn located_text(app: &mut App, text: &str) -> Rect {
    let found = app.locate_text(text);
    assert!(!found.is_empty(), "the text {text:?} isn't on screen");
    found[0]
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
    config.database_path = PathBuf::from("data").join(avalio_lite::platform::config::DATABASE_FILE_NAME);
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

/// The sample class, and Xoán Vidal, who left the only other class there was: he is in none now.
fn with_a_pupil_in_no_class() -> Fixture {
    use avalio_lite::gradebook::{classes, pupils};
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        common::sample_class(connection);
        let spare = classes::create_class(connection, "Spare").unwrap();
        let pupil = pupils::add_new_pupil(connection, spare, "Xoán Vidal").unwrap();
        pupils::remove_from_class(connection, spare, pupil).unwrap();
        classes::delete_class(connection, spare).unwrap();
    }
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
                    .blobs(colours.get("accent", dark), 6, app.area(200.0))
                    .into_iter()
                    .filter(|blob| blob.height > app.px(20.0))
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
        let accent = image.blobs(colours.get("accent", dark), 6, app.area(200.0));
        let near = app.px(4.0);
        assert!(accent.iter().any(|blob| blob.x.abs_diff(create.x) < near), "Create class is the primary button");
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
            let green = image.blobs(colours.get("pass-soft", dark), 4, app.area(300.0));
            let rose = image.blobs(colours.get("fail-soft", dark), 4, app.area(300.0));
            assert_eq!(green.len(), passing, "pass pills: {green:?}");
            assert_eq!(rose.len(), failing, "fail pills: {rose:?}");
            for pill in green.iter().chain(&rose) {
                assert_on_screen(image, *pill, "a grade pill");
            }
            // pills of one row line up, give or take a pixel or two
            let mut tops: Vec<u32> = green.iter().chain(&rose).map(|pill| pill.y).collect();
            tops.sort_unstable();
            let gap = app.px(4.0);
            let rows = 1 + tops.windows(2).filter(|pair| pair[1] - pair[0] > gap).count();
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

    a_wide_pupil_column();
    a_long_note();
    enter_moves_down_a_column();
}

/// The longest name in the sample, given to Uxía so the pupil column is too narrow for it.
const LONG_PUPIL_NAME: &str = "Uxía Fernández de la Iglesia Castelao";

/// The pupil column dragged to 400px by its edge, with real mouse messages: the long name shows whole.
fn a_wide_pupil_column() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        use avalio_lite::gradebook::pupils;
        let connection = state.connection().unwrap();
        let class = common::sample_class(connection);
        let uxia = pupils::list_class_pupils(connection, class).unwrap()[4].id;
        pupils::rename_pupil(connection, uxia, LONG_PUPIL_NAME).unwrap();
    }
    state.shutdown();
    relative_paths(&fixture);
    let mut app = App::start(&config(&fixture), "en");
    // the edges in sheet order: the pupil column's is the first
    app.drag_nth("Column width", 0, 400.0 - 210.0);
    let state = app.state();
    assert_eq!(state["name_width"], 400, "{state}");
    every_look(&mut app, "sheet-wide-names", |app, image, _, _| {
        let edge = app.locate("Column width")[0];
        let names = app.locate("Pupil");
        let uxia = names.iter().max_by_key(|name| name.y).copied().unwrap();
        assert_on_screen(image, uxia, "the long name");
        // a field is only wider than its column when its text doesn't fit
        assert!(uxia.right() <= edge.right(), "the whole name fits: {uxia:?}, edge {edge:?}");
    });
    app.quit();
    let mut reopened = App::start(&config(&fixture), "en");
    let state = reopened.state();
    assert_eq!(state["name_width"], 400, "the width is kept after a restart: {state}");
    reopened.quit();
}

/// A note longer than its cell, typed for real and with End pressed: the end of it shows.
fn a_long_note() {
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.type_into("Ana Pérez, Notes", "Brings a note from home about missing the trip to Santiago");
    app.press_end();
    for dark in [false, true] {
        app.resize(NORMAL.0, NORMAL.1);
        app.theme(dark);
        let theme = if dark { "dark" } else { "light" };
        app.snapshot(&format!("sheet-long-note-{theme}-{}x{}", NORMAL.0, NORMAL.1));
    }
    app.quit();
}

/// A mark typed and confirmed with Enter: the cell below has the focus border and takes the next mark.
fn enter_moves_down_a_column() {
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.type_into("Ana Pérez, Exam 1", "7");
    app.press_enter();
    for dark in [false, true] {
        app.resize(NORMAL.0, NORMAL.1);
        app.theme(dark);
        let theme = if dark { "dark" } else { "light" };
        app.snapshot(&format!("sheet-enter-{theme}-{}x{}", NORMAL.0, NORMAL.1));
    }
    app.send(json!({ "cmd": "type", "text": "5" }));
    app.settle();
    let state = app.state();
    assert_eq!(cells(&state, 0)[0]["text"], "7", "{state}");
    assert_eq!(cells(&state, 1)[0]["text"], "5", "Enter moved to Brais's exam: {state}");
    app.quit();
}

/// The column panel's buttons, by what a screen reader says (the Move buttons' keys included).
const PANEL_CONTROLS: [&str; 6] =
    ["Close", "Previous column (Alt+←)", "Next column (Alt+→)", "Move left (Ctrl+←)", "Move right (Ctrl+→)", "Delete"];
const VK_LEFT: u16 = 0x25;

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_column_panel_and_menus() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Exam 1");
    assert_eq!(app.state()["panel_index"], 0);
    every_look(&mut app, "column-panel", |app, image, _, _| {
        for control in PANEL_CONTROLS.iter().chain(&["Bonus test"]) {
            assert_on_screen(image, located(app, control), control);
        }
    });
    app.click("Next column (Alt+→)");
    assert_eq!(app.state()["panel_index"], 1, "a real click on › showed the next column");
    app.click("Previous column (Alt+←)");
    assert_eq!(app.state()["panel_index"], 0);
    app.click("Move right (Ctrl+→)");
    let state = app.state();
    assert_eq!(state["columns"][1], "Exam 1", "a real click moved the column: {state}");
    app.press_with_control(VK_LEFT);
    let state = app.state();
    assert_eq!(state["columns"][0], "Exam 1", "Ctrl+← moved it back: {state}");
    // Escape closes the panel after a click in it, from the real window too
    app.press_escape();
    assert_eq!(app.state()["panel_index"], -1, "Escape closed the panel");

    // resizing closes an open menu, so it opens again at each size
    every_look_after(
        &mut app,
        "add-column-menu",
        |app| app.click("+ Add column"),
        |app, image, _, _| {
            for option in ["Test", "Info", "Final grade", "Average", "Rounded"] {
                let found = app.locate_in_menu(option);
                assert_eq!(found.len(), 1, "{option}");
                assert_on_screen(image, found[0], option);
            }
        },
    );
    app.click_in_menu("Info");
    assert_eq!(app.state()["columns"].as_array().unwrap().len(), 7, "a real click added a column");
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    app.click("Exam 1");
    let image = galician_look(&mut app, "column-panel");
    for english in PANEL_CONTROLS {
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

/// A window that lost the focus (another one came to the front during the run) is still captured
/// as the user sees it while working in it: the open panel's Title field drawn focused.
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_snapshot_shows_the_window_active() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Exam 1");
    // the smallest size, where the field's outline is the biggest share of the screen
    app.resize(MINIMUM.0, MINIMUM.1);
    app.theme(false);
    app.window.lose_focus();
    app.settle();
    assert_eq!(app.state()["active"], false, "WM_KILLFOCUS made the window inactive");
    app.snapshot("column-panel-light-1000x600");
    app.quit();
}

/// The pupil menu marks a pupil in no class and offers to delete them; the keyboard works in the
/// real window's popups too (headless tests can't tell how a real popup takes the focus).
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_pupil_menu_and_the_menus_from_the_keyboard() {
    let _turn = desktop_support::turn();
    let fixture = with_a_pupil_in_no_class();
    // an open popup keeps its place when the window is resized, so each look starts the app afresh
    for dark in [false, true] {
        for size in [NORMAL, MINIMUM] {
            let mut app = App::start(&config(&fixture), "en");
            app.resize(size.0, size.1);
            app.theme(dark);
            app.click("More actions");
            app.click_in_popup("Add from another class…");
            let theme = if dark { "dark" } else { "light" };
            let image = app.snapshot(&format!("pupil-menu-{theme}-{}x{}", size.0, size.1));
            for control in ["Xoán Vidal", "Delete Xoán Vidal"] {
                let found = app.locate_in_popup(control);
                assert_eq!(found.len(), 1, "{control}");
                assert_on_screen(&image, found[0], control);
            }
            let name = app.locate_in_popup("Xoán Vidal")[0];
            let delete = app.locate_in_popup("Delete Xoán Vidal")[0];
            assert!(delete.x >= name.x, "the delete button sits beside the name, not before it");
            app.quit();
        }
    }
    let mut app = App::start(&config(&fixture), "en");
    app.click("More actions");
    app.click_in_popup("Add from another class…");
    app.answer_dialogs(&[], &[], &[true]);
    app.click_in_popup("Delete Xoán Vidal");
    assert!(app.locate("Delete Xoán Vidal").is_empty(), "the emptied menu closed");
    app.quit();

    // the one above deleted the pupil
    let fixture = with_a_pupil_in_no_class();
    let mut app = App::start(&config(&fixture), "gl");
    app.resize(MINIMUM.0, MINIMUM.1);
    app.click(&common::galician("More actions"));
    app.click_in_popup(&common::galician("Add from another class…"));
    let image = galician_look(&mut app, "pupil-menu");
    let delete = app.locate_in_popup("Eliminar a Xoán Vidal");
    assert_eq!(delete.len(), 1);
    assert_on_screen(&image, delete[0], "Eliminar a Xoán Vidal");
    app.quit();

    // the add-column menu from the keyboard, in the real window
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    let before = app.state()["columns"].as_array().unwrap().len();
    app.click("+ Add column");
    app.press_down();
    app.press_enter();
    let columns = app.state()["columns"].as_array().unwrap().clone();
    assert_eq!(columns.len(), before + 1, "Down then Enter added the second option, Info");
    app.click("+ Add column");
    app.press_escape();
    assert!(app.locate("Rounded").is_empty(), "Escape closed the menu");
    assert_eq!(app.state()["columns"].as_array().unwrap().len(), before + 1);
    app.click("+ Add column");
    app.press_enter();
    assert_eq!(app.state()["columns"].as_array().unwrap().len(), before + 2, "the first option is the one on");
    app.quit();
}

/// A workbook for the sample class: Ana's exam as it already is, a new pupil, and a column the
/// class doesn't have, so the report has every part.
fn sample_import(fixture: &Fixture) -> PathBuf {
    let path = fixture.path("marks.xlsx");
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let tab = workbook.add_worksheet();
    for (column, title) in (0u16..).zip(["Pupil", "Exam 1", "Phone"]) {
        tab.write_string(0, column, title).unwrap();
    }
    tab.write_string(1, 0, "Ana Pérez").unwrap();
    tab.write_number(1, 1, 8).unwrap();
    tab.write_string(2, 0, "Xoán Vidal").unwrap();
    tab.write_number(2, 1, 7).unwrap();
    tab.write_string(2, 2, "600 000 000").unwrap();
    workbook.save(&path).unwrap();
    path
}

/// The sheet's green status banner: the one wide patch of the pass colour (grade pills are small).
fn green_banner(colours: &ThemeColours, image: &Image, dark: bool) -> Rect {
    let wide = image.blobs(colours.get("pass-soft", dark), 4, 300).into_iter().find(|blob| blob.width > 400);
    wide.expect("no green banner on screen")
}

/// Closes the open sheet menu without changing anything: its export, cancelled.
fn close_with_a_cancelled_export(app: &mut App, export: &str) {
    app.cancel_next_save();
    app.click_in_popup(export);
}

/// The sheet's "⋯" menu while every pupil is already in the class.
const SHEET_MENU: [&str; 7] = [
    "Duplicate class",
    "Duplicate with pupils",
    "Duplicate with pupils and marks",
    "Hide weights & scales",
    "Export to Excel…",
    "Import from Excel…",
    "Delete class",
];

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_sheet_menu_and_an_import() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_sample_class();
    let workbook = sample_import(&fixture);
    let mut app = App::start(&config(&fixture), "en");
    // resizing closes an open menu (resizing_closes_the_sheet_menus), so it opens at each size
    let open_menu = |app: &mut App| app.click("More actions");
    every_look_after(&mut app, "sheet-menu", open_menu, |app, image, _, _| {
        for option in SHEET_MENU {
            let found = app.locate_in_popup(option);
            assert_eq!(found.len(), 1, "{option}");
            assert_on_screen(image, found[0], option);
        }
        close_with_a_cancelled_export(app, "Export to Excel…");
    });
    assert!(app.state()["status"].as_str().unwrap().is_empty(), "a cancelled export says nothing");
    app.click("More actions");
    app.answer_dialogs(&[], &[&workbook], &[]);
    app.click_in_popup("Import from Excel…");
    let state = app.state();
    let report = "Imported 2 pupils (1 new) and 1 mark or note into 2º ESO A. Left out: Phone.";
    assert_eq!(state["status"], report, "{state}");
    assert_eq!(state["rows"].as_array().unwrap().len(), 6, "a real click imported Xoán");
    every_look(&mut app, "sheet-imported", |_, image, dark, _| {
        assert_on_screen(image, green_banner(&colours, image, dark), "the import report");
    });
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    app.resize(MINIMUM.0, MINIMUM.1);
    app.click(&common::galician("More actions"));
    let image = galician_look(&mut app, "sheet-menu");
    for english in SHEET_MENU {
        let label = common::galician(english);
        let found = app.locate_in_popup(&label);
        assert_eq!(found.len(), 1, "{label}");
        assert_on_screen(&image, found[0], &label);
    }
    app.answer_dialogs(&[], &[&workbook], &[]);
    app.click_in_popup(&common::galician("Import from Excel…"));
    let report = "Importáronse 2 alumnos e 0 notas ou observacións en 2º ESO A. Quedou fóra: Phone.";
    assert_eq!(app.state()["status"], report);
    let image = galician_look(&mut app, "sheet-imported");
    assert_on_screen(&image, green_banner(&colours, &image, false), "the import report");
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
        // the page scrolls; in the smallest window the shortcuts card ends below the fold
        if size == NORMAL {
            for row in ["Lock", "Switch between light and dark", "Collapse or expand the sidebar"] {
                assert_on_screen(image, located(app, row), row);
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
            assert_on_screen(image, located(app, "Encrypt with this PIN"), "Encrypt with this PIN");
        },
    );
    app.scroll_main(40);
    every_look_after(
        &mut app,
        "settings-folders",
        |app| app.scroll_main(-40),
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
    assert!(fixture.path("moved-logs").join(avalio_lite::platform::config::LOG_FILE_NAME).is_file());

    let mut app = App::start(&config(&fixture), "gl");
    app.click("Axustes");
    let image = galician_look(&mut app, "settings");
    for control in ["Claro", "Escuro", "Galego", "English"] {
        assert_on_screen(&image, located(&mut app, control), control);
    }
    app.scroll("Galego", -4);
    let image = galician_look(&mut app, "settings-shortcuts");
    for row in ["Bloquear", "Cambiar entre claro e escuro", "Amosar ou agochar pesos e escalas", "Clase anterior"] {
        assert_on_screen(&image, located(&mut app, row), row);
    }
    // a row recording its new keys: the prompt and Cancel, in the longer language at the minimum size
    app.click_nth("Cambiar", 1);
    let image = galician_look(&mut app, "settings-recording");
    assert_on_screen(&image, located(&mut app, "Cancelar"), "Cancelar");
    app.click("Cancelar");
    app.scroll_main(-40);
    let image = galician_look(&mut app, "settings-folders");
    assert_on_screen(&image, located(&mut app, "Abrir outra base de datos…"), "Abrir outra base de datos…");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_backup_card() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    // a backup made elsewhere with another PIN, so the card asks for it
    let other = Fixture::new();
    let mut state = other.set_up();
    common::sample_class(state.connection().unwrap());
    state.set_pin(PIN, PIN).unwrap();
    let backup = other.path("avalio-lite-backup-2026-10-28.avalio");
    state.export_backup(&backup).unwrap();
    state.shutdown();

    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    app.scroll_main(-40);
    // relative to the scratch folder the app runs in, so the banner shows a stable path
    app.answer_saves(&[Path::new("saved.avalio")]);
    app.click("Export everything…");
    let state = app.state();
    assert_eq!(state["status"], "Backup saved to saved.avalio", "{state}");
    assert!(fixture.path("saved.avalio").is_file());
    every_look_after(
        &mut app,
        "settings-backup",
        |app| app.scroll_main(-40),
        |app, image, _, _| {
            for button in ["Export everything…", "Restore from a backup…", "Add classes from a backup…"] {
                assert_on_screen(image, located(app, button), button);
            }
        },
    );
    app.answer_dialogs(&[], &[&backup], &[]);
    app.click("Restore from a backup…");
    every_look_after(
        &mut app,
        "settings-backup-pin",
        |app| app.scroll_main(-40),
        |app, image, _, _| {
            for control in ["Backup PIN", "Continue", "Cancel"] {
                assert_on_screen(image, located(app, control), control);
            }
        },
    );
    app.click("Cancel");
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    app.click("Axustes");
    app.scroll_main(-40);
    app.answer_dialogs(&[], &[&backup], &[]);
    app.click("Restaurar unha copia…");
    // the prompt opens below the buttons: at the minimum size it needs the page scrolled down
    app.resize(MINIMUM.0, MINIMUM.1);
    app.scroll_main(-40);
    let image = galician_look(&mut app, "settings-backup-pin");
    for control in ["Engadir clases dunha copia…", "PIN da copia", "Continuar", "Cancelar"] {
        assert_on_screen(&image, located(&mut app, control), control);
    }
    app.click("Cancelar");
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
fn the_licence_screen() {
    let _turn = desktop_support::turn();
    let fixture = with_database();

    let mut app = App::start_unlicensed(&config(&fixture), "en");
    assert_eq!(app.state()["licensed"], false);
    every_look(&mut app, "licence", |app, image, _, _| {
        for control in ["Licence key", "Activate", "Back up my data"] {
            assert_on_screen(image, located(app, control), control);
        }
    });
    // the longest message, in the least room
    app.send(json!({ "cmd": "licence", "refuse": "already_activated", "status": 409 }));
    app.type_into("Licence key", "BRN-79YN-44XQ-W025-377Z");
    app.click("Activate");
    app.resize(MINIMUM.0, MINIMUM.1);
    app.theme(false);
    let image = app.snapshot("licence-in-use-elsewhere-light-1000x600");
    assert_on_screen(&image, located(&mut app, "Back up my data"), "Back up my data");
    assert_eq!(app.state()["licensed"], false);

    app.send(json!({ "cmd": "licence" }));
    app.click("Activate");
    let state = app.state();
    assert_eq!(state["licensed"], true, "real clicks activated the key: {state}");
    assert!(!app.locate("Settings").is_empty(), "the sheet shows");
    app.quit();

    let mut app = App::start_unlicensed(&config(&fixture), "gl");
    let image = galician_look(&mut app, "licence");
    for control in ["Clave de licenza", "Activar", "Facer copia dos meus datos"] {
        assert_on_screen(&image, located(&mut app, control), control);
    }
    app.send(json!({ "cmd": "licence", "refuse": "already_activated", "status": 409 }));
    app.type_into("Clave de licenza", "BRN-79YN-44XQ-W025-377Z");
    app.click("Activar");
    let image = galician_look(&mut app, "licence-in-use-elsewhere");
    assert_on_screen(&image, located(&mut app, "Facer copia dos meus datos"), "Facer copia dos meus datos");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_update_bar() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    for language in ["en", "gl"] {
        let words = |english: &str| if language == "en" { english.to_owned() } else { common::galician(english) };
        let mut app = App::start(&config(&fixture), language);
        // a fixed version, so the bar's text is the same on every run
        app.send(json!({"cmd": "release", "version": "9.0.0"}));
        app.click(&words("Settings"));
        scroll_into_view(&mut app, &words("Check for updates now"));
        app.click(&words("Check for updates now"));
        assert!(app.state()["update"].as_str().unwrap().contains("9.0.0"), "a real click looked for it");
        app.click("2º ESO A");
        let (update_now, whats_new) = (words("Update now"), words("What's new"));
        let check_bar = |app: &mut App, image: &Image| {
            assert_on_screen(image, located(app, &update_now), "Update now");
            assert_on_screen(image, located(app, &whats_new), "What's new");
        };
        if language == "en" {
            every_look_after(&mut app, "update-bar", |_| {}, |app, image, _, _| check_bar(app, image));
            app.click("Hide");
            assert_eq!(app.state()["update"], "", "a real click hid the bar");
        } else {
            let image = galician_look(&mut app, "update-bar");
            check_bar(&mut app, &image);
        }
        app.quit();
    }
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_licence_card_and_the_offline_warning() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();

    let mut app = App::start(&config(&fixture), "en");
    app.click("Settings");
    every_look_after(
        &mut app,
        "settings-licence",
        |app| scroll_into_view(app, "Check for updates now"),
        |app, image, _, _| {
            assert_on_screen(image, located(app, "Check now"), "Check now");
            assert_on_screen(image, located_text(app, "Licensed"), "Licensed");
        },
    );
    app.click("Check now");
    let state = app.state();
    assert_eq!(state["status"], "Licence confirmed.", "a real click checked the licence: {state}");
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    app.click("Axustes");
    app.resize(MINIMUM.0, MINIMUM.1);
    scroll_into_view(&mut app, "Buscar actualizacións agora");
    let image = galician_look(&mut app, "settings-licence");
    assert_on_screen(&image, located(&mut app, "Comprobar agora"), "Comprobar agora");
    assert_on_screen(&image, located_text(&mut app, "Con licenza"), "Con licenza");
    app.quit();

    // the warning names a day counted from today, so it is checked for room, not against a baseline
    for language in ["en", "gl"] {
        let mut app = App::start_offline_soon(&config(&fixture), language);
        let warning = app.state()["status"].as_str().unwrap().to_owned();
        assert!(warning.contains("internet"), "{warning}");
        for dark in [false, true] {
            for size in [NORMAL, MINIMUM] {
                app.resize(size.0, size.1);
                app.theme(dark);
                let image = app.capture();
                assert_on_screen(&image, located_text(&mut app, &warning), "the offline warning");
            }
        }
        app.quit();
    }
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

/// The app as users run it: `main`, `app::run` and the real Windows dialogs, which every other test
/// replaces. Tab to the first-run screen's "Browse…", press it, close the folder dialog that opens,
/// then close the window: the app must quit cleanly.
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_real_dialogs_open_and_close() {
    use desktop_support::window::{AppWindow, dialog_of};
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};
    /// The first-run card is centred; its database "Browse…" sits this far from the window's
    /// centre (logical pixels, from the setup baseline). The app driver can't help here: this is
    /// the real app, which has no driver.
    const BROWSE_FROM_CENTRE: (f64, f64) = (204.0, 33.0);
    const LOGICAL_WIDTH: f64 = 1280.0;
    /// How long a closed dialog may take to go, and how often it is told to close meanwhile.
    const DIALOG_CLOSE_WAIT: Duration = Duration::from_secs(10);
    const DIALOG_CLOSE_AGAIN: Duration = Duration::from_secs(2);

    /// Kills the app if the test fails before it quits, so no window is left behind.
    struct Running(Child);
    impl Drop for Running {
        fn drop(&mut self) {
            if matches!(self.0.try_wait(), Ok(None)) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }

    let _turn = desktop_support::turn();
    let fixture = Fixture::new();
    let mut app = Running(
        Command::new(desktop_support::app_exe())
            .env(avalio_lite::platform::config::CONFIG_DIR_ENV, fixture.path("config"))
            .current_dir(fixture.root.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("the app starts"),
    );
    let pid = app.0.id();
    let window = AppWindow::of_process(pid, Duration::from_secs(45)).expect("the app shows its window");
    std::thread::sleep(Duration::from_millis(800));
    let (width, height) = window.client_size();
    let scale = f64::from(width) / LOGICAL_WIDTH;
    let x = f64::from(width) / 2.0 + BROWSE_FROM_CENTRE.0 * scale;
    let y = f64::from(height) / 2.0 + BROWSE_FROM_CENTRE.1 * scale;
    window.click(x as i32, y as i32);
    let dialog = dialog_of(pid, &window, Duration::from_secs(10)).expect("the folder dialog opens");
    // Windows' dialog may take a while to go, and one that has only just appeared can miss a close
    // message, so the close is repeated now and then until it is gone (or the wait runs out)
    let deadline = Instant::now() + DIALOG_CLOSE_WAIT;
    let mut closed_at = Instant::now();
    dialog.close();
    while dialog_of(pid, &window, Duration::ZERO).is_some() {
        assert!(Instant::now() < deadline, "closing it cancels it (still open after {DIALOG_CLOSE_WAIT:?})");
        if closed_at.elapsed() >= DIALOG_CLOSE_AGAIN {
            dialog.close();
            closed_at = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    window.close();
    let status = app.0.wait().expect("the app exits");
    assert!(status.success(), "a cancelled dialog and the close button quit cleanly: {status}");
}

/// The sidebar folded to its 64 px icon rail, by its button and by Ctrl+B: the sheet starts at
/// the rail's edge, and every entry of the rail is on screen.
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_collapsed_sidebar() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Collapse the sidebar");
    assert_eq!(app.state()["sidebar_collapsed"], true);
    every_look(&mut app, "sidebar-collapsed", |app, image, dark, _| {
        let theme = if dark { "Light mode" } else { "Dark mode" };
        for control in ["Expand the sidebar", "2º ESO A", "New class", theme, "Settings"] {
            assert_on_screen(image, located(app, control), control);
        }
        // the sheet's card starts right of the rail: its "+ Add pupil" sits well left of where the
        // expanded sidebar alone would end
        let rail = app.px(64.0);
        // the class's rail entry, not the sheet's title of the same name
        let entry = app.locate("2º ESO A").into_iter().min_by_key(|found| found.x).unwrap();
        assert!(entry.right() <= rail, "the class entry stays inside the rail: {entry:?}");
        // the sheet's title starts at the rail's edge plus the page's padding, where the expanded
        // sidebar alone would reach past
        let title = app.locate("2º ESO A").into_iter().max_by_key(|found| found.x).unwrap();
        assert!(title.x < app.px(236.0), "the sheet took the room: its title is at {title:?}");
    });
    app.click("Expand the sidebar");
    assert_eq!(app.state()["sidebar_collapsed"], false);
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    app.click(&common::galician("Collapse the sidebar"));
    let image = galician_look(&mut app, "sidebar-collapsed");
    for control in ["Expand the sidebar", "New class", "Settings"] {
        let label = common::galician(control);
        assert_on_screen(&image, located(&mut app, &label), &label);
    }
    app.click(&common::galician("Expand the sidebar"));
    app.quit();
}

/// Resizing the window closes whichever sheet menu is open, instead of leaving it cut off at the
/// place it opened at (regression).
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn resizing_closes_the_sheet_menus() {
    type OpenMenu = fn(&mut App);
    let _turn = desktop_support::turn();
    let fixture = with_a_pupil_in_no_class();
    let mut app = App::start(&config(&fixture), "en");
    for dark in [false, true] {
        app.theme(dark);
        let menus: [(&str, OpenMenu); 3] = [
            ("the ⋯ menu", |app| app.click("More actions")),
            ("the add-column menu", |app| app.click("+ Add column")),
            ("the pupil menu", |app| {
                app.click("More actions");
                app.click_in_popup("Add from another class…");
            }),
        ];
        for (menu, open) in menus {
            app.resize(NORMAL.0, NORMAL.1);
            open(&mut app);
            assert!(app.popup_is_open(), "{menu} opened");
            app.resize(MINIMUM.0, MINIMUM.1);
            assert!(!app.popup_is_open(), "resizing closed {menu} (dark: {dark})");
        }
    }
    app.quit();
}

/// The real exe opens fully inside the work area of its screen, centred (src/app/placement.rs).
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_window_opens_inside_the_work_area() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CMONITORS};
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let app = App::start(&config(&fixture), "en");
    let (window, work) = (app.window.outer_rect(), app.window.work_area());
    eprintln!(
        "window ({}, {})–({}, {}) in the work area ({}, {})–({}, {})",
        window.left, window.top, window.right, window.bottom, work.left, work.top, work.right, work.bottom
    );
    assert!(
        window.left >= work.left
            && window.top >= work.top
            && window.right <= work.right
            && window.bottom <= work.bottom,
        "the window reaches past the work area"
    );
    let centre = |low: i32, high: i32| (low + high) as f64 / 2.0;
    assert!((centre(window.left, window.right) - centre(work.left, work.right)).abs() <= 2.0, "not centred across");
    assert!((centre(window.top, window.bottom) - centre(work.top, work.bottom)).abs() <= 2.0, "not centred down");
    // SAFETY: a plain query.
    if unsafe { GetSystemMetrics(SM_CMONITORS) } > 1 {
        // starting on another screen means moving the user's own mouse there, which these tests never do
        eprintln!("note: more than one monitor; the start on a second one is covered by tests/placement.rs only");
    } else {
        eprintln!("note: one monitor here, so the start on a second one isn't checked");
    }
    app.quit();
}

/// The smallest real-world screen (1366×768 at 125 %, taskbar out: 1092×582 logical, where the app
/// opens maximised with its minimum lowered): every screen's controls stay reachable.
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn small_screens_stay_usable() {
    const SMALL: (u32, u32) = (1092, 582);
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
        assert_on_screen(&image, located(&mut app, control), control);
    }

    app.click("Exam 1");
    let image = app.capture();
    for control in PANEL_CONTROLS.iter().chain(&["Bonus test"]) {
        assert_on_screen(&image, located(&mut app, control), control);
    }
    app.click("Close");

    app.click("Settings");
    let image = app.snapshot(&format!("settings-light-{}x{}", SMALL.0, SMALL.1));
    for control in ["Light", "Dark", "Galego", "English"] {
        assert_on_screen(&image, located(&mut app, control), control);
    }
    app.scroll_main(-10);
    let image = app.capture();
    assert_on_screen(&image, located(&mut app, "Encrypt with this PIN"), "Encrypt with this PIN");
    app.scroll_main(-40);
    let image = app.capture();
    assert_on_screen(&image, located(&mut app, "Open another database…"), "Open another database…");
    for (index, what) in ["the database's Move", "the logs' Move"].into_iter().enumerate() {
        assert_on_screen(&image, app.locate("Move…")[index], what);
    }
    app.quit();

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
        assert_on_screen(&image, located(&mut app, key), key);
    }
    app.quit();
}
