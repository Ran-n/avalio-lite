// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.580780
//+ Revised: 	2026/11/24 16:26:03.509777
// -----------------------------------------------------------------------------

//! Export to and import from Excel: the menu entries, the file dialogs and the reports afterwards.

use crate::driver::{click, count, english, error_text, gradebook, type_into};
use crate::fixtures::{cell_text, seeded};
use crate::screens::{export_through_the_menu, import_file, import_through_the_menu};
use avalio_lite::error::AppError;
use avalio_lite::i18n::{Message, SpreadsheetMessage};
use avalio_lite::window::dialogs::FileKind;
use slint::Model;
use std::fs;

#[test]
fn export_to_excel_saves_the_sheet_and_says_where() {
    let ui = seeded();
    let destination = ui.fixture.path("1A.xlsx");
    export_through_the_menu(&ui, Some(&destination));
    assert!(destination.is_file());
    let shown = destination.display().to_string();
    assert_eq!(
        ui.session.window.get_status_message(),
        english(&Message::Spreadsheet(SpreadsheetMessage::SheetExported { class: "1A", path: &shown }))
    );
    assert!(ui.session.window.get_error_message().is_empty());
    let documents = dirs::document_dir().unwrap_or_default();
    assert_eq!(
        ui.dialogs.save_requests.borrow()[0],
        (FileKind::Spreadsheet, documents, "1A.xlsx".to_owned()),
        "Documents, the first time"
    );

    export_through_the_menu(&ui, Some(&ui.fixture.path("again.xlsx")));
    assert_eq!(ui.dialogs.save_requests.borrow()[1].1, ui.fixture.root.path(), "then where the last one went");
}

#[test]
fn the_proposed_file_name_leaves_out_what_windows_refuses() {
    let ui = seeded();
    type_into(&ui, "Class name", "1A/1B: maths");
    export_through_the_menu(&ui, None);
    assert_eq!(ui.dialogs.save_requests.borrow()[0].2, "1A-1B- maths.xlsx");
}

#[test]
fn a_cancelled_export_does_nothing() {
    let ui = seeded();
    export_through_the_menu(&ui, None);
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(
        fs::read_dir(ui.fixture.root.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "xlsx"))
            .count(),
        0
    );
}

#[test]
fn a_failed_export_shows_the_error() {
    let ui = seeded();
    let destination = ui.fixture.path("no such folder").join("1A.xlsx");
    export_through_the_menu(&ui, Some(&destination));
    let banner = ui.session.window.get_error_message().to_string();
    assert!(banner.starts_with(&format!("Couldn't save {}. ", destination.display())), "{banner}");
    assert!(ui.session.window.get_status_message().is_empty());

    export_through_the_menu(&ui, Some(&ui.fixture.path("1A.xlsx")));
    assert!(ui.session.window.get_error_message().is_empty(), "a later export that works clears it");
    assert!(!ui.session.window.get_status_message().is_empty());
    assert_eq!(
        ui.dialogs.save_requests.borrow()[1].1,
        dirs::document_dir().unwrap_or_default(),
        "a failed export's folder isn't remembered"
    );
}

/// Was: nothing cleared the green banner, so "Saved 1A to …" stayed next to later work.
#[test]
fn regression_the_export_status_clears_on_the_next_action() {
    let ui = seeded();
    export_through_the_menu(&ui, Some(&ui.fixture.path("1A.xlsx")));
    assert!(!ui.session.window.get_status_message().is_empty());
    type_into(&ui, "Class name", "1A bis");
    assert!(ui.session.window.get_status_message().is_empty(), "a sheet action clears it");

    export_through_the_menu(&ui, Some(&ui.fixture.path("1A.xlsx")));
    click(&ui, "More actions");
    click(&ui, "Hide weights & scales");
    assert!(ui.session.window.get_status_message().is_empty(), "so does a preference");
}

#[test]
fn import_from_excel_fills_the_sheet_and_reports() {
    let ui = seeded();
    let file = import_file(
        &ui,
        &[&["Pupil", "Exam", "Phone", "", "Final grade"], &["Ana", "7", "600", "", "9"], &["Carla", "8", "", "x", ""]],
    );
    import_through_the_menu(&ui, Some(&file));
    assert_eq!(*ui.dialogs.file_kinds.borrow(), [FileKind::Spreadsheet]);
    assert!(ui.session.window.get_error_message().is_empty(), "{}", ui.session.window.get_error_message());
    assert_eq!(
        ui.session.window.get_status_message(),
        "Imported 2 pupils (1 new) and 2 marks or notes into 1A. Left out: Phone, column D (no title), Final grade."
    );
    assert!(ui.dialogs.questions.borrow().is_empty(), "nothing to replace, so nothing to ask");
    assert_eq!(gradebook(&ui).get_rows().row_count(), 3, "Carla joins the sheet");
    assert_eq!(cell_text(&ui, 0, 0), "7");
    assert_eq!(cell_text(&ui, 2, 0), "8");
    assert_eq!(gradebook(&ui).get_classes().row_data(0).unwrap().pupil_count, 3, "the class list counts her too");
}

#[test]
fn import_reports_skipped_error_cells_and_singulars() {
    let ui = seeded();
    let file = import_file(&ui, &[&["Pupil", "Exam"], &["Ana", "#DIV/0!"]]);
    import_through_the_menu(&ui, Some(&file));
    assert_eq!(
        ui.session.window.get_status_message(),
        "Imported 1 pupil and 0 marks or notes into 1A. 1 cell with an Excel error was skipped."
    );
}

#[test]
fn overwriting_asks_first_and_no_changes_nothing() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "3");
    let file = import_file(&ui, &[&["Pupil", "Exam"], &["Ana", "9"], &["Bea", "6"]]);
    ui.dialogs.answer_next(false);
    import_through_the_menu(&ui, Some(&file));
    assert_eq!(
        *ui.dialogs.questions.borrow(),
        [english(&Message::Spreadsheet(SpreadsheetMessage::ConfirmImportOverwrite { count: 1, class: "1A" }))],
        "only Ana's 3 is replaced; Bea's empty cell isn't"
    );
    assert_eq!(cell_text(&ui, 0, 0), "3");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 1, "no change at all");
    assert!(ui.session.window.get_status_message().is_empty());

    ui.dialogs.answer_next(true);
    import_through_the_menu(&ui, Some(&file));
    assert_eq!((cell_text(&ui, 0, 0), cell_text(&ui, 1, 0)), ("9".to_owned(), "6".to_owned()));
    assert_eq!(ui.session.window.get_status_message(), "Imported 2 pupils and 2 marks or notes into 1A.");
}

#[test]
fn an_import_the_database_refuses_shows_the_error_and_changes_nothing() {
    let ui = seeded();
    ui.session
        .state
        .borrow()
        .connection()
        .unwrap()
        .execute_batch("CREATE TEMP TRIGGER no_cells BEFORE INSERT ON cells BEGIN SELECT RAISE(ABORT, 'test'); END")
        .unwrap();
    let file = import_file(&ui, &[&["Pupil", "Exam"], &["Carla", "8"]]);
    import_through_the_menu(&ui, Some(&file));
    assert!(!ui.session.window.get_error_message().is_empty());
    assert!(ui.session.window.get_status_message().is_empty(), "nothing was imported");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "Carla isn't left behind");
}

#[test]
fn a_cancelled_import_does_nothing() {
    let ui = seeded();
    import_through_the_menu(&ui, None);
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2);
}

#[test]
fn an_unreadable_file_shows_the_error() {
    let ui = seeded();
    let file = ui.fixture.path("notes.xlsx");
    fs::write(&file, "not a workbook").unwrap();
    import_through_the_menu(&ui, Some(&file));
    let banner = ui.session.window.get_error_message().to_string();
    assert!(banner.starts_with(&format!("{} isn't a workbook that can be read.", file.display())), "{banner}");
    assert!(ui.session.window.get_status_message().is_empty());

    let header_only = import_file(&ui, &[&["Pupil", "Exam"]]);
    import_through_the_menu(&ui, Some(&header_only));
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::NoPupilsInFile(header_only)));

    let twice = import_file(&ui, &[&["Pupil"], &["Eva"], &["eva"]]);
    import_through_the_menu(&ui, Some(&twice));
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::DuplicatePupilInFile("eva".into())));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "nothing was added");
}
