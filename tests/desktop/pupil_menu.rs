// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.101138
//+ Revised: 	2026/11/24 17:54:08.448158
// -----------------------------------------------------------------------------

//! The pupil menu, which a pupil in no class is offered from the sheet's "⋯" menu: how it looks at
//! each size, deleting the pupil from it, and how it looks in Galician.

use crate::desktop_support::{self, App};

use super::fixtures::{config, with_a_pupil_in_no_class};
use super::screens::{MINIMUM, NORMAL, assert_on_screen, galician_look};

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_pupil_menu_looks_right_at_each_size() {
    let _turn = desktop_support::turn();
    let fixture = with_a_pupil_in_no_class();
    // an open popup keeps its place when the window is resized, so each look starts the app afresh
    for dark in [false, true] {
        for size in [NORMAL, MINIMUM] {
            let mut app = App::start(&config(&fixture), "en");
            app.resize(size.0, size.1);
            app.theme(dark);
            app.open_add_from_another_class();
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
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn deleting_a_pupil_from_the_pupil_menu_removes_them() {
    let _turn = desktop_support::turn();
    let fixture = with_a_pupil_in_no_class();
    let mut app = App::start(&config(&fixture), "en");
    app.open_add_from_another_class();
    app.answer_dialogs(&[], &[], &[true]);
    app.click_in_popup("Delete Xoán Vidal");
    assert!(app.locate("Delete Xoán Vidal").is_empty(), "the emptied menu closed");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_pupil_menu_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_a_pupil_in_no_class();
    let mut app = App::start(&config(&fixture), "gl");
    app.resize(MINIMUM.0, MINIMUM.1);
    app.click(&crate::common::galician("More actions"));
    app.click_in_popup(&crate::common::galician("Add from another class…"));
    let image = galician_look(&mut app, "pupil-menu");
    let delete = app.locate_in_popup("Eliminar a Xoán Vidal");
    assert_eq!(delete.len(), 1);
    assert_on_screen(&image, delete[0], "Eliminar a Xoán Vidal");
    app.quit();
}
