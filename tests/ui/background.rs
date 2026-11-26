// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:30.739681
//+ Revised: 	2026/11/23 13:00:30.739681
// -----------------------------------------------------------------------------

//! Background work that can't start or can't reach the window, and sheet rows that are gone.

use crate::common::licence::{FakeLicence, Mode};
use crate::common::{Fixture, Ui};
use crate::driver::{click, count, english, error_text, gradebook};
use crate::fixtures::{seeded, with_second_class};
use crate::screens::{
    break_saving_the_settings, error_on_screen, import_file, import_through_the_menu, licence, licence_calls,
    open_licence_settings, with_a_worker_that_cannot_start,
};
use avalio_lite::error::AppError;
use avalio_lite::i18n::{AppMessage, Language, Message, SpreadsheetMessage};
use avalio_lite::ui::ColumnType as UiColumnType;
use slint::{Model, SharedString};

/// Clears the status and error lines, so the next action's own message can be told apart.
fn view_clear(ui: &Ui) {
    ui.session.window.set_status_message(SharedString::new());
    ui.session.window.set_error_message(SharedString::new());
}

#[test]
fn a_licence_check_that_cannot_be_started_ends_the_wait_and_says_so() {
    let ui = with_a_worker_that_cannot_start();
    open_licence_settings(&ui);
    let before = licence_calls(&ui);
    click(&ui, "Check now");
    assert_eq!(licence_calls(&ui), before, "nothing ran");
    assert!(!licence(&ui).get_busy());
    assert_eq!(error_on_screen(&ui), english(&Message::App(AppMessage::BackgroundFailed)));
    assert!(licence(&ui).get_licensed(), "the licence stands");
}

// ---------------------------------------------------------------------------------------------
// Background work with no event loop to come back on (as after the window closed)

#[test]
fn a_background_result_that_cannot_reach_the_window_is_dropped() {
    use avalio_lite::window::worker::{self, ThreadWorker, Worker};
    let ui = Ui::with_database();
    let worker = ThreadWorker::default();
    let reached = std::rc::Rc::new(std::cell::Cell::new(false));
    let back = reached.clone();
    worker::run(&worker, &ui.session.window, || 1, move |_, _| back.set(true)).unwrap();
    worker.join();
    assert!(!reached.get(), "the headless backend has no event loop to hand it over");
}

#[test]
fn a_daily_licence_answer_that_cannot_reach_the_window_changes_nothing() {
    let computer = FakeLicence::activated();
    let client = computer.client_checking_every(std::time::Duration::from_millis(10));
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let ui = Ui::open_licensed(fixture, Language::English, computer, client.clone());
    let checks = avalio_lite::licence::ui::start_checks(&ui.session.window, &client).unwrap();
    let before = licence_calls(&ui);
    ui.licence.server.set_mode(Mode::Refuse("licence_revoked", 403));
    ui.licence.advance(chrono::TimeDelta::hours(25));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while licence_calls(&ui) == before {
        assert!(std::time::Instant::now() < deadline, "the daily check never ran");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // waits for the check under way, and so for its answer to be handed over (or not)
    checks.stop();
    assert!(licence(&ui).get_licensed(), "the screen is as it was");
}

// ---------------------------------------------------------------------------------------------
// The sheet: rows and columns it no longer has, and settings that can't be saved

#[test]
fn an_import_says_how_many_rows_had_no_name() {
    let ui = seeded();
    let file = import_file(&ui, &[&["Pupil", "Exam"], &["", "5"], &["Dora", "6"]]);
    import_through_the_menu(&ui, Some(&file));
    let status = ui.session.window.get_status_message().to_string();
    assert!(
        status.contains(&english(&Message::Spreadsheet(SpreadsheetMessage::ImportUnnamedRows { count: 1 }))),
        "{status}"
    );
}

#[test]
fn acting_on_a_row_or_column_the_sheet_no_longer_has_is_said_not_dropped() {
    let ui = seeded();
    let refused = error_text(&AppError::NotEditable);
    let sheet = gradebook(&ui);
    let refuses = |what: &str, act: &dyn Fn()| {
        view_clear(&ui);
        act();
        assert_eq!(ui.session.window.get_error_message(), refused, "{what}");
    };
    refuses("a cell", &|| sheet.invoke_cell_edited(99, 99, "5".into()));
    refuses("a column to delete", &|| sheet.invoke_delete_column(9999));
    refuses("a column to resize", &|| sheet.invoke_set_column_width(99, 120));
    refuses("a pupil to remove", &|| sheet.invoke_remove_pupil(9999));
    refuses("a pupil in no class to delete", &|| sheet.invoke_delete_pupil_in_no_class(99));
    assert!(ui.dialogs.questions.borrow().is_empty(), "nothing was asked about something that isn't there");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 0);
}

#[test]
fn a_class_opens_even_when_it_cannot_be_remembered_for_the_next_start() {
    let ui = with_second_class();
    break_saving_the_settings(&ui);
    click(&ui, "2B");
    assert_eq!(gradebook(&ui).get_class_name(), "2B", "the class is open");
    assert!(!ui.session.window.get_error_message().is_empty(), "the settings failure is said");
    assert!(!gradebook(&ui).get_save_failed(), "nothing typed is at stake");
}

/// The name columns come with the class; the "Add column" menu doesn't offer them, and asking
/// for one anyway is refused.
#[test]
fn a_second_name_column_cannot_be_added() {
    let ui = seeded();
    let before = gradebook(&ui).get_columns().row_count();
    for name in [UiColumnType::FirstName, UiColumnType::Surname] {
        view_clear(&ui);
        gradebook(&ui).invoke_add_column(name);
        assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::NotEditable), "{name:?}");
    }
    assert_eq!(gradebook(&ui).get_columns().row_count(), before);
}
