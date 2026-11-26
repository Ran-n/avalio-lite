// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.753950
//+ Revised: 	2026/11/23 13:00:32.753950
// -----------------------------------------------------------------------------

//! The saved acknowledgement: when the sheet says changes are saved, and when it says they are not.

use crate::common::{Fixture, Ui};
use crate::driver::{
    click, click_element, click_menu_option, count, english, gradebook, key, press_enter, replace_text, shows_text,
    type_into, type_text, with_control,
};
use crate::fixtures::{
    encrypted, encrypted_fixture, name_fields, open_folder_settings, open_settings, seeded, unlock, with_keys,
    with_second_class, with_shortcut_config,
};
use crate::screens::{
    backup_file, drag_edge, export_through_the_menu, import_file, import_through_the_menu, open_backup_settings,
    unlicensed, width_edges,
};
use avalio_lite::i18n::{self, Language, Message, SheetMessage};
use avalio_lite::ui::ColumnType as UiColumnType;
use slint::platform::Key;
use slint::{Model, SharedString};
use std::fs;

// ---------------------------------------------------------------------------------------------
// The acknowledgement that changes are saved

const SAVES_BY_ITSELF: &str = "Saves as you type";

const SAVED: &str = "✓ Saved";

const NOT_SAVED: &str = "Not saved";

fn save_count(ui: &Ui) -> i32 {
    gradebook(ui).get_save_count()
}

/// Runs `change` and checks the summary line acknowledged it as one more written change.
fn assert_acknowledged(ui: &Ui, what: &str, change: impl FnOnce()) {
    let before = save_count(ui);
    change();
    assert!(ui.session.window.get_error_message().is_empty(), "{what}: {}", ui.session.window.get_error_message());
    assert!(save_count(ui) > before, "{what} wasn't acknowledged");
    assert!(gradebook(ui).get_saved() && !gradebook(ui).get_save_failed(), "{what}");
}

#[test]
fn the_sheet_says_it_saves_as_you_type_until_something_changes() {
    let ui = with_second_class();
    assert!(shows_text(&ui, SAVES_BY_ITSELF));
    assert!(!shows_text(&ui, SAVED) && !shows_text(&ui, NOT_SAVED));
    // looking around writes nothing, so nothing is acknowledged
    click(&ui, "2B");
    click(&ui, "1A");
    gradebook(&ui).invoke_open_column_panel(0);
    assert_eq!(gradebook(&ui).get_panel_index(), 0);
    gradebook(&ui).invoke_close_column_panel();
    let destination = ui.fixture.path("1A.xlsx");
    export_through_the_menu(&ui, Some(&destination));
    assert!(destination.is_file());
    assert_eq!(save_count(&ui), 0);
    assert!(shows_text(&ui, SAVES_BY_ITSELF));
}

#[test]
fn a_typed_mark_is_acknowledged_as_saved() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    assert!(shows_text(&ui, SAVED), "the summary line says so");
    assert!(!shows_text(&ui, SAVES_BY_ITSELF));
    assert_eq!(save_count(&ui), 1);
    type_into(&ui, "Bea, Exam", "3");
    assert_eq!(save_count(&ui), 2, "each change is acknowledged");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 2, "and both are in the database");
}

#[test]
fn every_kind_of_change_is_acknowledged() {
    let ui = seeded();
    assert_acknowledged(&ui, "a class name", || type_into(&ui, "Class name", "1B"));
    assert_acknowledged(&ui, "a pupil's name", || {
        click_element(&name_fields(&ui, "First name")[0]);
        replace_text(&ui, "Ana María");
    });
    assert_acknowledged(&ui, "a new pupil", || {
        click(&ui, "+ Add pupil");
        type_text(&ui, "Carmen");
        press_enter(&ui);
        press_enter(&ui);
    });
    assert_acknowledged(&ui, "a new column", || gradebook(&ui).invoke_add_column(UiColumnType::Info));
    assert_acknowledged(&ui, "a weight", || {
        click(&ui, "Exam");
        type_into(&ui, "Weight", "4");
    });
    assert_acknowledged(&ui, "a column width", || drag_edge(&width_edges(&ui)[2], 80.0));
    assert_acknowledged(&ui, "a deleted column", || {
        let notes = gradebook(&ui).get_columns().iter().last().unwrap().id;
        ui.dialogs.answer_next(true);
        gradebook(&ui).invoke_delete_column(notes);
    });
    gradebook(&ui).invoke_close_column_panel();
    assert_acknowledged(&ui, "an import", || {
        let file = import_file(&ui, &[&["Pupil", "Exam"], &["Dora", "6"]]);
        import_through_the_menu(&ui, Some(&file));
    });
    assert_acknowledged(&ui, "a duplicate", || {
        click(&ui, "More actions");
        click_menu_option(&ui, "Duplicate class");
    });
}

#[test]
fn an_action_that_writes_nothing_is_not_acknowledged() {
    let ui = seeded();
    let first = gradebook(&ui).get_columns().row_data(0).unwrap().id;
    gradebook(&ui).invoke_move_column(first, -1);
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(save_count(&ui), 0, "the first column can't go further left, so nothing was written");
    assert!(shows_text(&ui, SAVES_BY_ITSELF));
}

#[test]
fn a_change_that_fails_says_not_saved_until_the_next_one_works() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "x");
    assert!(!ui.session.window.get_error_message().is_empty());
    assert!(shows_text(&ui, NOT_SAVED), "the field shows a weight the database doesn't hold");
    assert_eq!(save_count(&ui), 0);
    type_into(&ui, "Weight", "4");
    assert!(shows_text(&ui, SAVED));
    assert!(!shows_text(&ui, NOT_SAVED));
}

#[test]
fn a_refused_value_put_back_no_longer_says_not_saved() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "x");
    assert!(gradebook(&ui).get_save_failed());
    key(&ui, Key::Escape);
    assert!(!gradebook(&ui).get_save_failed(), "the field shows the saved weight again");
    assert!(shows_text(&ui, SAVES_BY_ITSELF));
}

#[test]
fn a_file_that_cannot_be_imported_does_not_say_not_saved() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    let file = ui.fixture.path("notes.xlsx");
    fs::write(&file, "not a workbook").unwrap();
    import_through_the_menu(&ui, Some(&file));
    assert!(!ui.session.window.get_error_message().is_empty());
    assert!(!gradebook(&ui).get_save_failed(), "nothing typed was at stake");
    assert!(shows_text(&ui, SAVED));
}

#[test]
fn another_database_starts_unacknowledged() {
    let other = Fixture::new();
    other.set_up().shutdown();
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    assert_eq!(save_count(&ui), 1);
    open_folder_settings(&ui);
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open another database…");
    assert_eq!(save_count(&ui), 0);
    assert!(!gradebook(&ui).get_saved() && !gradebook(&ui).get_save_failed());
    drop(ui);

    // and so does one unlocked again
    let ui = encrypted();
    unlock(&ui);
    assert!(gradebook(&ui).invoke_create_class("1A".into()));
    assert_eq!(save_count(&ui), 1);
    with_control(&ui, "l");
    unlock(&ui);
    assert_eq!(save_count(&ui), 0);
    assert!(shows_text(&ui, SAVES_BY_ITSELF));
}

#[test]
fn a_restored_backup_and_added_classes_are_acknowledged() {
    let (_other, file) = backup_file(None);
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    ui.dialogs.answer_next(true);
    click(&ui, "Restore from a backup…");
    assert!(gradebook(&ui).get_saved(), "a restore");
    assert_eq!(save_count(&ui), 1);

    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Add classes from a backup…");
    assert!(ui.session.window.get_error_message().is_empty(), "{}", ui.session.window.get_error_message());
    assert_eq!(save_count(&ui), 2, "added classes");
    click(&ui, "7A");
    assert!(shows_text(&ui, SAVED), "back on the sheet");
}

#[test]
fn ctrl_s_says_everything_is_saved() {
    let ui = seeded();
    click(&ui, "Ana, Exam");
    with_control(&ui, "s");
    assert_eq!(
        ui.session.window.get_status_message(),
        english(&Message::Sheet(SheetMessage::EverythingSaved)),
        "from a cell"
    );
    assert_eq!(save_count(&ui), 0, "it writes nothing");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 0, "and types nothing into the cell");
    open_settings(&ui);
    assert!(ui.session.window.get_status_message().is_empty());
    with_keys(&ui, false, "S");
    assert_eq!(
        ui.session.window.get_status_message(),
        english(&Message::Sheet(SheetMessage::EverythingSaved)),
        "in Settings"
    );
    click(&ui, "Galego");
    with_control(&ui, "s");
    assert_eq!(
        ui.session.window.get_status_message(),
        i18n::text(Language::Galician, &Message::Sheet(SheetMessage::EverythingSaved))
    );
    // with Shift it is another key combination, left alone
    ui.session.window.set_status_message(SharedString::new());
    with_keys(&ui, true, "S");
    assert!(ui.session.window.get_status_message().is_empty());
}

#[test]
fn ctrl_s_does_nothing_on_the_lock_first_run_and_licence_screens() {
    // one at a time: only one window may be open
    let screens: [fn() -> Ui; 3] = [encrypted, || Ui::open(Fixture::new()), unlicensed];
    for open in screens {
        let ui = open();
        with_control(&ui, "s");
        assert!(ui.session.window.get_status_message().is_empty());
        assert!(ui.session.window.get_error_message().is_empty());
    }
}

#[test]
fn a_shortcut_the_user_put_on_ctrl_s_wins() {
    let fixture = encrypted_fixture();
    with_shortcut_config(&fixture, &[("lock", "Ctrl+S")]);
    let ui = Ui::open(fixture);
    unlock(&ui);
    with_control(&ui, "s");
    assert!(ui.session.window.get_locked(), "Ctrl+S locks, as the settings file says");
    assert!(ui.session.window.get_status_message().is_empty());
}
