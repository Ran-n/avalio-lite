// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.772599
//+ Revised: 	2026/11/24 17:54:08.962670
// -----------------------------------------------------------------------------

//! The sheet's "⋯" menu: how it looks at each size, a cancelled export, importing a workbook from it
//! and the green report that the import leaves, in English and in Galician.

use std::path::Path;

use crate::desktop_support::pixels::{Image, Rect};
use crate::desktop_support::{self, App, ThemeColours};

use super::fixtures::{SHEET_MENU, config, sample_import, with_sample_class};
use super::screens::{MINIMUM, assert_on_screen, every_look, every_look_after, galician_look};

/// The sheet's Import from Excel, answered with `workbook` at the file picker.
fn import_from_excel(app: &mut App, workbook: &Path) {
    app.open_more_actions();
    app.answer_dialogs(&[], &[workbook], &[]);
    app.click_in_popup("Import from Excel…");
}

/// The sheet's green status banner: the one wide patch of the pass colour (grade pills are small).
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the look under test, light or dark, which every desktop screen is checked in"
)]
fn green_banner(colours: &ThemeColours, image: &Image, dark: bool) -> Rect {
    let wide = image.blobs(colours.get("pass-soft", dark), 4, 300).into_iter().find(|blob| blob.width > 400);
    wide.expect("no green banner on screen")
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_sheet_menu_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    // resizing closes an open menu (resizing_closes_the_sheet_menus), so it opens at each size
    every_look_after(&mut app, "sheet-menu", App::open_more_actions, |app, image, _, _| {
        for option in SHEET_MENU {
            let found = app.locate_in_popup(option);
            assert_eq!(found.len(), 1, "{option}");
            assert_on_screen(image, found[0], option);
        }
        app.cancel_export("Export to Excel…");
    });
    assert!(app.state()["status"].as_str().unwrap().is_empty(), "a cancelled export says nothing");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn importing_from_excel_adds_the_pupil_and_marks() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let workbook = sample_import(&fixture);
    let mut app = App::start(&config(&fixture), "en");
    import_from_excel(&mut app, &workbook);
    let state = app.state();
    let report = "Imported 2 pupils (1 new) and 1 mark or note into 2º ESO A. Left out: Phone.";
    assert_eq!(state["status"], report, "{state}");
    assert_eq!(state["rows"].as_array().unwrap().len(), 6, "a real click imported Xoán");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_import_report_banner_looks_right() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_sample_class();
    let workbook = sample_import(&fixture);
    let mut app = App::start(&config(&fixture), "en");
    import_from_excel(&mut app, &workbook);
    every_look(&mut app, "sheet-imported", |_, image, dark, _| {
        assert_on_screen(image, green_banner(&colours, image, dark), "the import report");
    });
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_sheet_menu_and_import_in_galician() {
    let _turn = desktop_support::turn();
    let colours = ThemeColours::load();
    let fixture = with_sample_class();
    let workbook = sample_import(&fixture);
    // the Galician menu and report come after the English import, which the sheet already holds:
    // its pupil is in the class and its mark is in, so the report counts no new mark
    let mut app = App::start(&config(&fixture), "en");
    import_from_excel(&mut app, &workbook);
    app.quit();

    let mut app = App::start(&config(&fixture), "gl");
    app.resize(MINIMUM.0, MINIMUM.1);
    app.click(&crate::common::galician("More actions"));
    let image = galician_look(&mut app, "sheet-menu");
    for english in SHEET_MENU {
        let label = crate::common::galician(english);
        let found = app.locate_in_popup(&label);
        assert_eq!(found.len(), 1, "{label}");
        assert_on_screen(&image, found[0], &label);
    }
    app.answer_dialogs(&[], &[&workbook], &[]);
    app.click_in_popup(&crate::common::galician("Import from Excel…"));
    let report = "Importáronse 2 alumnos e 0 notas ou observacións en 2º ESO A. Quedou fóra: Phone.";
    assert_eq!(app.state()["status"], report);
    let image = galician_look(&mut app, "sheet-imported");
    assert_on_screen(&image, green_banner(&colours, &image, false), "the import report");
    app.quit();
}
