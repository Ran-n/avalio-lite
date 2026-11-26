// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.332977
//+ Revised: 	2026/11/24 17:54:07.604485
// -----------------------------------------------------------------------------

//! The first-run setup screen: how it looks in English and Galician, and creating the database from it.

use crate::common::Fixture;
use crate::desktop_support::{self, App, ThemeColours};

use super::fixtures::{SHOWN_FOLDER, config};
use super::screens::{assert_on_screen, every_look, galician_look};

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_setup_screen_looks_right() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = Fixture::new();
    let mut app = App::start(&config(&fixture), "en");
    app.type_into("Database folder", SHOWN_FOLDER);
    app.type_into("Log folder", SHOWN_FOLDER);
    every_look(&mut app, "setup", |app, image, dark, _| {
        // the brand tile and the primary button are the accent colour, both whole
        let accent: Vec<_> = image
            .blobs(colours.get("accent", dark), 6, app.area(200.0))
            .into_iter()
            .filter(|blob| blob.height > app.px(20.0))
            .collect();
        assert_eq!(accent.len(), 2, "brand tile and Create button: {accent:?}");
        let create = app.located("Create database");
        assert_on_screen(image, create, "Create database");
        let open = app.located("Open an existing database…");
        assert_on_screen(image, open, "Open an existing database");
    });
    app.quit();

    let fixture = Fixture::new();
    let mut app = App::start(&config(&fixture), "gl");
    app.type_into("Cartafol da base de datos", SHOWN_FOLDER);
    app.type_into("Cartafol dos rexistros", SHOWN_FOLDER);
    let image = galician_look(&mut app, "setup");
    let create = app.located("Crear base de datos");
    assert_on_screen(&image, create, "Crear base de datos");
    let open = app.located("Abrir unha base de datos existente…");
    assert_on_screen(&image, open, "Abrir…");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn creating_the_database_from_the_setup_screen() {
    let _turn = desktop_support::turn();
    let fixture = Fixture::new();
    let mut app = App::start(&config(&fixture), "en");
    app.type_into("Database folder", &fixture.path("data").display().to_string());
    app.type_into("Log folder", &fixture.path("logs").display().to_string());
    app.click("Create database");
    let state = app.state();
    assert_eq!(state["setup_required"], false, "{state}");
    assert!(fixture.database().is_file(), "a real click created the database");
    app.quit();
}
