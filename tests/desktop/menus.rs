// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.951149
//+ Revised: 	2026/11/24 17:54:08.236565
// -----------------------------------------------------------------------------

//! The sheet's menus close when the window is resized, instead of staying where they opened.

use crate::desktop_support::{self, App};

use super::fixtures::{config, with_a_pupil_in_no_class};
use super::screens::{MINIMUM, NORMAL};

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
            ("the ⋯ menu", App::open_more_actions),
            ("the add-column menu", App::open_add_column_menu),
            ("the pupil menu", App::open_add_from_another_class),
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
