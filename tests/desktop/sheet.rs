// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.462388
//+ Revised: 	2026/11/26 05:25:10.912424
// -----------------------------------------------------------------------------

//! The class sheet: how it looks in light and dark, and in Galician, and a mark typed into it.

use crate::desktop_support::{self, App, ThemeColours};
use serde_json::Value;

use super::fixtures::{cells, config, with_sample_class};
use super::screens::{NORMAL, assert_on_screen, every_look, galician_dark_look, galician_look};

/// The final grades of the first five pupils: how many pass and how many fail.
fn grade_pills(state: &Value) -> (usize, usize) {
    // the final grades are pills (the rounding's pills are past the window's right edge, behind the
    // two name columns)
    let pills: Vec<Value> = (0..5).map(|row| cells(state, row)[3].clone()).collect();
    let passing = pills.iter().filter(|cell| cell["tone"] == "pass").count();
    let failing = pills.iter().filter(|cell| cell["tone"] == "fail").count();
    assert!(passing > 0 && failing > 0, "the sample has both: {pills:?}");
    (passing, failing)
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_sheet_looks_right_in_light_and_dark_and_in_galician() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    let (passing, failing) = grade_pills(&app.state());

    every_look(&mut app, "sheet", |app, image, dark, size| {
        assert_on_screen(image, app.located("+ Add column"), "+ Add column");
        assert_on_screen(image, app.located("More actions"), "More actions");
        if size == NORMAL {
            // every final grade is a pill: pass pills green, fail pills rose
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

    // the Galician sheet is captured after Daniel's exam was typed to 9, as a user would have left it
    app.type_into("Daniel Souto, Exam 1", "9");
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    let image = galician_look(&mut app, "sheet");
    assert_on_screen(&image, app.located("+ Engadir columna"), "+ Engadir columna");
    let image = galician_dark_look(&mut app, "sheet");
    assert_on_screen(&image, app.located("+ Engadir columna"), "+ Engadir columna");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_typed_mark_changes_the_final_grade() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    // a real mark, typed into the real cell: Daniel's exam goes from 4 to 9 and he passes
    app.type_into("Daniel Souto, Exam 1", "9");
    let after = app.state();
    assert_eq!(cells(&after, 3)[0]["text"], "9");
    assert_eq!(cells(&after, 3)[3]["tone"], "pass", "{after}");
    app.quit();
}
