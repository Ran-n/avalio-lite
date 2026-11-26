// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.185892
//+ Revised: 	2026/11/24 17:54:07.402923
// -----------------------------------------------------------------------------

//! Creating the first class: how the screen with no class looks, the Create button, and the sidebar's
//! field that makes a second class on Enter.

use crate::desktop_support::{self, App, ThemeColours};
use serde_json::json;

use super::fixtures::{config, with_database};
use super::screens::{assert_on_screen, every_look, galician_look};

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_screen_with_no_class_looks_right() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_database();
    let mut app = App::start(&config(&fixture), "en");
    every_look(&mut app, "no-class", |app, image, dark, _| {
        let create = app.located("Create class");
        assert_on_screen(image, create, "Create class");
        let accent = image.blobs(colours.get("accent", dark), 6, app.area(200.0));
        let near = app.px(4.0);
        assert!(accent.iter().any(|blob| blob.x.abs_diff(create.x) < near), "Create class is the primary button");
    });
    app.quit();

    let empty = with_database();
    let mut app = App::start(&config(&empty), "gl");
    let image = galician_look(&mut app, "no-class");
    assert_on_screen(&image, app.located("Crear clase"), "Crear clase");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_create_class_button_makes_the_first_class() {
    let _turn = desktop_support::turn();
    let fixture = with_database();
    let mut app = App::start(&config(&fixture), "en");
    app.type_into("Class name, e.g. 2º ESO A", "1A");
    app.click("Create class");
    let state = app.state();
    assert_eq!(state["class_name"], "1A", "{state}");
    assert_eq!(state["classes"], json!(["1A"]));
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_sidebar_field_makes_a_class_on_enter() {
    let _turn = desktop_support::turn();
    let fixture = with_database();
    let mut app = App::start(&config(&fixture), "en");
    // the sidebar's field shows once there is a class to sit beside it
    app.type_into("Class name, e.g. 2º ESO A", "1A");
    app.click("Create class");
    app.type_into("New class", "2B");
    app.press_enter();
    let state = app.state();
    assert_eq!(state["class_name"], "2B", "{state}");
    app.quit();
}
