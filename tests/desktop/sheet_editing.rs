// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.591355
//+ Revised: 	2026/11/24 12:35:14.525045
// -----------------------------------------------------------------------------

//! Editing the sheet: the name columns dragged wider, a long note read to its end, Enter moving down
//! a column, and the "✓ Saved" indicator fading back to grey after a change.

use std::time::{Duration, Instant};

use crate::common::Fixture;
use crate::desktop_support::pixels::{Image, Rect, Rgb};
use crate::desktop_support::{self, App, ThemeColours};
use serde_json::json;

use super::fixtures::{FIRST_MARK, LONG_SURNAME, cells, config, relative_paths, with_sample_class};
use super::screens::{NORMAL, assert_on_screen, every_look};

/// `fixture` with the sample class, and the longest surname given to Uxía.
fn with_long_surname() -> Fixture {
    use avalio_lite::gradebook::pupils;
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let class = crate::common::sample_class(connection);
        let uxia = pupils::list_class_pupils(connection, class).unwrap()[4].id;
        pupils::set_surname(connection, uxia, LONG_SURNAME).unwrap();
    }
    state.shutdown();
    relative_paths(&fixture);
    fixture
}

/// The name columns dragged wider by their edges: the first name's edge, then the surname's.
fn drag_name_columns(app: &mut App) {
    app.drag_nth("Column width", 0, 200.0 - 160.0);
    app.drag_nth("Column width", 1, 400.0 - 210.0);
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn dragging_the_name_columns_sets_their_widths() {
    let _turn = desktop_support::turn();
    let fixture = with_long_surname();
    let mut app = App::start(&config(&fixture), "en");
    drag_name_columns(&mut app);
    let state = app.state();
    assert_eq!(state["column_widths"][0], 200, "{state}");
    assert_eq!(state["column_widths"][1], 400, "{state}");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_long_surname_shows_whole_in_the_wide_column() {
    let _turn = desktop_support::turn();
    let fixture = with_long_surname();
    let mut app = App::start(&config(&fixture), "en");
    drag_name_columns(&mut app);
    every_look(&mut app, "sheet-wide-names", |app, image, _, _| {
        let edge = app.locate("Column width")[1];
        // the header first, then the pupils' surname fields
        let surnames = app.locate("Surname");
        let uxia = surnames.iter().max_by_key(|name| name.y).copied().unwrap();
        assert_on_screen(image, uxia, "the long surname");
        // a field is only wider than its column when its text doesn't fit
        assert!(uxia.right() <= edge.right(), "the whole surname fits: {uxia:?}, edge {edge:?}");
    });
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_name_column_widths_survive_a_restart() {
    let _turn = desktop_support::turn();
    let fixture = with_long_surname();
    let mut app = App::start(&config(&fixture), "en");
    drag_name_columns(&mut app);
    app.quit();
    let mut reopened = App::start(&config(&fixture), "en");
    let state = reopened.state();
    assert_eq!(state["column_widths"][0], 200, "the widths are kept after a restart: {state}");
    assert_eq!(state["column_widths"][1], 400, "the widths are kept after a restart: {state}");
    reopened.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_long_note_shows_its_end() {
    let _turn = desktop_support::turn();
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        use avalio_lite::gradebook::columns;
        let connection = state.connection().unwrap();
        let class = crate::common::sample_class(connection);
        let notes = columns::list_columns(connection, class).unwrap().last().unwrap().id;
        columns::move_column_to(connection, notes, FIRST_MARK).unwrap();
    }
    state.shutdown();
    relative_paths(&fixture);
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

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn enter_moves_down_a_column() {
    let _turn = desktop_support::turn();
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

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_saved_indicator_fades_back_to_grey() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_sample_class();
    for (language, saved) in [("en", "✓ Saved"), ("gl", "✓ Gardado")] {
        let mut app = App::start(&config(&fixture), language);
        app.resize(NORMAL.0, NORMAL.1);
        for (dark, mark) in [(false, "6"), (true, "7")] {
            app.theme(dark);
            app.type_into("Ana Pérez, Exam 1", mark);
            let image = app.capture();
            let indicator = app.located_text(saved);
            assert_on_screen(&image, indicator, saved);
            let muted = colours.get("muted", dark);
            let grey = pixels_of(&image, indicator, muted);
            assert!(grey > 0, "the settled indicator is grey ({language}, dark: {dark})");
            assert!(lights_up(&mut app, indicator, muted), "right after a change it is lit ({language}, dark: {dark})");
            let image = app.capture();
            assert_eq!(pixels_of(&image, indicator, muted), grey, "grey again ({language}, dark: {dark})");
            assert_eq!(pixels_of(&image, indicator, colours.get("pass", dark)), 0, "no green is left ({language})");
        }
        app.quit();
    }
}

/// Whether the settled `indicator` leaves `muted` after a change. Typing a whole mark into the field
/// takes longer than the fade, so a few more characters are typed (the mark becomes 6.5, then 6.51)
/// and the window is watched straight after each: one look at a fixed moment can come too early or
/// too late on a busy machine.
fn lights_up(app: &mut App, indicator: Rect, muted: Rgb) -> bool {
    let grey = |image: &Image| pixels_of(image, indicator, muted);
    [".5", "1", "1"].iter().any(|text| {
        let settled = grey(&app.capture());
        app.send(json!({ "cmd": "type", "text": text }));
        let end = Instant::now() + Duration::from_secs(3);
        while Instant::now() < end {
            if grey(&app.glimpse()) < settled {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    })
}

/// How many of `area`'s pixels are `colour`, give or take the smoothing at a letter's edge.
fn pixels_of(image: &Image, area: Rect, colour: Rgb) -> usize {
    let across = area.x..area.right().min(image.width);
    (area.y..area.bottom().min(image.height))
        .flat_map(|y| across.clone().map(move |x| (x, y)))
        .filter(|(x, y)| image.pixel(*x, *y).near(colour, 20))
        .count()
}
