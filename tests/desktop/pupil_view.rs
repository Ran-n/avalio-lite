// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:12:06.091147
//+ Revised: 	2026/11/25 11:43:12.783155
// -----------------------------------------------------------------------------

//! The pupil view: the list of pupils, one pupil's grades and the PIN between them, in every look, and
//! the whole round by real clicks. The pupil panel, where a pupil's picture is chosen, is here too.

use std::path::PathBuf;

use crate::common::{Fixture, galician};
use crate::desktop_support::pixels::Rgb;
use crate::desktop_support::{self, App, ThemeColours};

use super::fixtures::{PIN, config, with_locked_sample, with_sample_class};
use super::screens::{assert_on_screen, every_look, galician_look};

const PUPILS: [&str; 5] = ["Fernández, Uxía", "Iglesias, Carla", "Otero, Brais", "Pérez, Ana", "Souto, Daniel"];
/// The face's colour in the drawn photo, which no theme uses.
const FACE: Rgb = Rgb(224, 160, 110);

/// A drawn face (no one's photo): the baselines are committed.
fn drawn_photo(fixture: &Fixture) -> PathBuf {
    let path = fixture.path("photo.png");
    let inside = |x: u32, y: u32, at: (f32, f32), radius: f32| (x as f32 - at.0).hypot(y as f32 - at.1) < radius;
    let photo = image::RgbImage::from_fn(400, 300, |x, y| {
        if inside(x, y, (200.0, 130.0), 60.0) {
            image::Rgb([FACE.0, FACE.1, FACE.2])
        } else if inside(x, y, (200.0, 300.0), 95.0) {
            image::Rgb([70, 90, 150])
        } else {
            image::Rgb([190, 215, 200])
        }
    });
    photo.save(&path).unwrap();
    path
}

/// Gives the pupil the drawn photo, as a teacher does: their row's avatar, then the panel's button.
fn choose_photo(app: &mut App, fixture: &Fixture, pupil: &str) {
    let photo = drawn_photo(fixture);
    app.click(&format!("Picture of {pupil}"));
    app.answer_dialogs(&[], &[&photo], &[]);
    app.click("Choose a picture…");
}

fn open(app: &mut App) {
    app.open_more_actions();
    app.click_in_popup("Show a pupil their grades…");
    assert_eq!(app.state()["pupil_view"], "menu", "a real click opened it");
}

fn type_pin(app: &mut App, unlock: &str) {
    for key in PIN.split("").filter(|digit| !digit.is_empty()) {
        app.click(key);
    }
    app.click(unlock);
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_pupil_panel_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Picture of Brais Otero");
    every_look(&mut app, "pupil-panel", |app, image, _, _| {
        assert_on_screen(image, app.located("Choose a picture…"), "Choose a picture…");
    });
    choose_photo(&mut app, &fixture, "Brais Otero");
    every_look(&mut app, "pupil-panel-picture", |app, image, _, _| {
        for control in ["Change the picture…", "Remove the picture", "Close"] {
            assert_on_screen(image, app.located(control), control);
        }
        // the face, in the panel and (smaller) on the row
        assert_eq!(image.blobs(FACE, 24, 20).len(), 2, "the picture shows twice");
    });
    app.click("Close");
    assert!(app.locate("Change the picture…").is_empty());
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_list_of_pupils_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    choose_photo(&mut app, &fixture, "Carla Iglesias");
    app.click("Close");
    open(&mut app);
    every_look(&mut app, "pupil-view-menu", |app, image, _, _| {
        for control in PUPILS.into_iter().chain(["Back to the sheet"]) {
            assert_on_screen(image, app.located(control), control);
        }
        assert_eq!(image.blobs(FACE, 24, 20).len(), 1, "Carla's picture");
    });
    app.click("Back to the sheet");
    assert_eq!(app.state()["pupil_view"], "off");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn one_pupils_grades_look_right() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    choose_photo(&mut app, &fixture, "Brais Otero");
    app.click("Close");
    open(&mut app);
    app.click("Otero, Brais");
    assert_eq!(app.state()["pupil_shown"], "Brais Otero");
    every_look(&mut app, "pupil-view-grades", |app, image, dark, _| {
        for control in ["Back to the list", "Next pupil"] {
            assert_on_screen(image, app.located(control), control);
        }
        for grade in ["Exam 1: 6,5 / 10", "Project: no mark yet", "Report card: 5 / 10"] {
            assert_on_screen(image, app.located_text(grade), grade);
        }
        // Brais passes the exam and the report card, and fails the lab report and the final grade
        for tone in ["pass-soft", "fail-soft"] {
            assert_eq!(image.blobs(colours.get(tone, dark), 4, 300).len(), 2, "{tone} pills (dark: {dark})");
        }
    });
    app.click("Next pupil");
    assert_eq!(app.state()["pupil_shown"], "Ana Pérez", "with no PIN, straight to the next");
    app.click("Back to the list");
    assert_eq!(app.state()["pupil_view"], "menu");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_pin_page_looks_right_and_guards_the_pupils() {
    let _turn = desktop_support::turn();
    let fixture = with_locked_sample();
    let mut app = App::start(&config(&fixture), "en");
    type_pin(&mut app, "Unlock");
    open(&mut app);
    app.click("Souto, Daniel");
    let state = app.state();
    assert_eq!((&state["pupil_view"], &state["pupil_shown"]), (&"pin".into(), &"".into()), "{state}");
    every_look(&mut app, "pupil-view-pin", |app, image, _, _| {
        for key in ["1", "9", "Letters", "Erase", "Unlock", "Back to the list"] {
            assert_on_screen(image, app.located(key), key);
        }
        assert!(app.locate("Forgot the PIN?").is_empty(), "no way out of the list from here");
    });
    type_pin(&mut app, "Unlock");
    assert_eq!(app.state()["pupil_shown"], "Daniel Souto");
    app.click("Back to the list");
    app.click("Back to the sheet");
    assert_eq!(app.state()["pupil_view"], "pin", "leaving asks too");
    type_pin(&mut app, "Unlock");
    assert_eq!(app.state()["pupil_view"], "off");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_pupil_view_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_locked_sample();
    let mut app = App::start(&config(&fixture), "gl");
    type_pin(&mut app, "Desbloquear");
    app.click("Foto de Brais Otero");
    let image = galician_look(&mut app, "pupil-panel");
    let choose = galician("Choose a picture…");
    assert_on_screen(&image, app.located(&choose), &choose);
    app.click(&galician("Close"));
    app.click(&galician("More actions"));
    app.click_in_popup(&galician("Show a pupil their grades…"));
    let image = galician_look(&mut app, "pupil-view-menu");
    assert_on_screen(&image, app.located(&galician("Back to the sheet")), "Back to the sheet");
    app.click("Otero, Brais");
    let image = galician_look(&mut app, "pupil-view-pin");
    let back = galician("Back to the list");
    assert_on_screen(&image, app.located("Desbloquear"), "Desbloquear");
    assert_on_screen(&image, app.located(&back), &back);
    type_pin(&mut app, "Desbloquear");
    let image = galician_look(&mut app, "pupil-view-grades");
    let next = galician("Next pupil");
    assert_on_screen(&image, app.located(&next), &next);
    assert_on_screen(&image, app.located_text("Final grade: 4,7 / 10"), "the final grade");
    app.quit();
}
