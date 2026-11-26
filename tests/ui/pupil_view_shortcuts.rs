// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:19.594704
//+ Revised: 	2026/11/25 11:43:18.778251
// -----------------------------------------------------------------------------

//! What the keyboard shortcuts and the lock do while a pupil looks.

use crate::driver::{click, gradebook, shows, with_control};
use crate::fixtures::{with_keys, with_second_class};
use crate::pupil_screens::{grades, open_for, sample_with_pin, shown_name, stage, unlock_with};
use avalio_lite::ui::{PupilViewStage, Theme};
use slint::ComponentHandle;
use slint::platform::Key;

#[test]
fn the_sheets_shortcuts_do_nothing_while_a_pupil_looks() {
    let ui = with_second_class();
    open_for(&ui, "Ana");
    let dark = ui.session.window.global::<Theme>().get_dark();
    with_keys(&ui, false, Key::PageDown);
    for text in [",", "b", "h"] {
        with_keys(&ui, false, text);
    }
    with_keys(&ui, true, "D");
    with_control(&ui, "s");
    with_control(&ui, "l");
    assert_eq!(stage(&ui), PupilViewStage::Grades);
    assert_eq!(ui.session.window.global::<Theme>().get_dark(), dark);
    let gradebook = gradebook(&ui);
    assert_eq!(gradebook.get_class_name(), "1A");
    assert!(!gradebook.get_sidebar_collapsed() && gradebook.get_show_column_settings());
    assert!(ui.session.window.get_error_message().is_empty(), "Ctrl+L with no PIN says nothing either");
    assert!(ui.session.window.get_status_message().is_empty());
    click(&ui, "Back to the list");
    click(&ui, "Back to the sheet");
    assert!(shows(&ui, "More actions"), "still on the sheet, not in Settings");
    with_keys(&ui, false, Key::PageDown);
    assert_eq!(gradebook.get_class_name(), "2B", "and the shortcuts work again at once");
}

#[test]
fn locking_puts_the_pupil_view_away() {
    let ui = sample_with_pin();
    open_for(&ui, "Pérez, Ana");
    with_control(&ui, "l");
    assert!(ui.session.window.get_locked());
    assert_eq!(stage(&ui), PupilViewStage::Off);
    assert!(shown_name(&ui).is_empty() && grades(&ui).is_empty());
    unlock_with(&ui, crate::driver::PIN);
    assert!(shows(&ui, "More actions"), "unlocked by the teacher, the sheet is back");
}
