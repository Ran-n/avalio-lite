// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.885992
//+ Revised: 	2026/11/24 17:54:09.064566
// -----------------------------------------------------------------------------

//! The sidebar folded to its 64 px icon rail: how the rail looks in light and dark, the Galician rail,
//! and collapsing and expanding the sidebar.

use crate::desktop_support::{self, App};

use super::fixtures::{config, with_sample_class};
use super::screens::{assert_on_screen, every_look, galician_look};

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_collapsed_sidebar_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Collapse the sidebar");
    every_look(&mut app, "sidebar-collapsed", |app, image, dark, _| {
        let theme = if dark { "Light mode" } else { "Dark mode" };
        for control in ["Expand the sidebar", "2º ESO A", "New class", theme, "Settings"] {
            assert_on_screen(image, app.located(control), control);
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
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn collapsing_and_expanding_the_sidebar() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Collapse the sidebar");
    assert_eq!(app.state()["sidebar_collapsed"], true);
    app.click("Expand the sidebar");
    assert_eq!(app.state()["sidebar_collapsed"], false);
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_collapsed_sidebar_in_galician() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "gl");
    app.click(&crate::common::galician("Collapse the sidebar"));
    let image = galician_look(&mut app, "sidebar-collapsed");
    for control in ["Expand the sidebar", "New class", "Settings"] {
        let label = crate::common::galician(control);
        assert_on_screen(&image, app.located(&label), &label);
    }
    app.click(&crate::common::galician("Expand the sidebar"));
    app.quit();
}
