// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.543974
//+ Revised: 	2026/11/23 13:00:34.543974
// -----------------------------------------------------------------------------

//! Import of whole files: a sheet not at A1, a file with the wrong extension, a failure
//! part-way, and ODS tables.

use crate::support::Cell::{Empty, Number, Text};
use crate::support::{Scratch, read, small_class, strings};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{classes, columns, pupils, sheet};
use avalio_lite::spreadsheet::import::{apply_import, plan_import};
use std::path::Path;

/// The sheet's first used cell needn't be A1: a blank first row and column (a title row left
/// empty, a margin column) leave the headers and names where they are.
#[test]
fn a_sheet_starting_below_and_right_of_a1_is_read_from_its_first_cell() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch
        .workbook("x.xlsx", &[vec![], vec![Empty, Text("Pupil"), Text("Exam")], vec![Empty, Text("Ana"), Number(6.0)]]);
    let summary = scratch.import(class, &path);
    assert_eq!((summary.pupils, summary.new_pupils, summary.marks), (1, 0, 1), "Ana matched, her mark read");
    assert!(summary.left_out.is_empty(), "{:?}", summary.left_out);
    let loaded = sheet::load_sheet(&scratch.connection, class).unwrap();
    let ana = loaded.rows.iter().find(|row| row.pupil.first_name == "Ana").unwrap();
    assert!(ana.values.contains(&"6".to_owned()), "{:?}", ana.values);
}

/// One name column with names written surname first, as school lists have them: each splits at its
/// comma, so "Pérez, Ana" finds the pupil Ana Pérez instead of making a second one.
#[test]
fn a_one_column_name_written_surname_first_finds_its_pupil() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    let ana = pupils::add_new_pupil(&scratch.connection, class, "Ana", "Pérez López").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("Pupil")], vec![Text("Pérez López, Ana")], vec![Text("Iglesias,Carla")], vec![Text("Xoán Carlos")]],
    );
    let summary = scratch.import(class, &path);
    assert_eq!((summary.pupils, summary.new_pupils), (3, 2));
    let names: Vec<(i64, String, String)> = pupils::list_class_pupils(&scratch.connection, class)
        .unwrap()
        .into_iter()
        .map(|pupil| (pupil.id, pupil.first_name, pupil.surname))
        .collect();
    assert_eq!(names[0], (ana, "Ana".to_owned(), "Pérez López".to_owned()), "matched, not duplicated");
    assert!(names.iter().any(|(_, first, surname)| first == "Carla" && surname == "Iglesias"));
    assert!(names.iter().any(|(_, first, surname)| first == "Xoán Carlos" && surname.is_empty()));
}

#[test]
fn a_header_only_file_has_no_pupils() {
    let scratch = Scratch::new();
    let path = scratch.workbook("x.xlsx", &[vec![Text("Pupil"), Text("Exam")]]);
    let error = read(&path).unwrap_err();
    assert!(matches!(&error, AppError::NoPupilsInFile(failed) if *failed == path), "{error:?}");
    let empty = scratch.workbook("empty.xlsx", &[]);
    assert!(matches!(read(&empty).unwrap_err(), AppError::NoPupilsInFile(_)));
}

#[test]
fn a_text_file_renamed_xlsx_is_a_read_error() {
    let scratch = Scratch::new();
    let path = scratch.path("notes.xlsx");
    std::fs::write(&path, "just some notes").unwrap();
    let error = read(&path).unwrap_err();
    assert!(matches!(&error, AppError::SpreadsheetRead { path: failed, .. } if *failed == path), "{error:?}");
    assert!(matches!(read(&scratch.path("gone.xlsx")).unwrap_err(), AppError::SpreadsheetRead { .. }));
}

#[test]
fn a_failure_mid_import_rolls_everything_back() {
    let scratch = Scratch::new();
    let (class, notes, _) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam"), Text("Notes")],
            vec![Text("Eva"), Number(5.0), Text("New")],
            vec![Text("Ana"), Number(6.0), Text("Late")],
        ],
    );
    let plan = plan_import(&scratch.connection, class, &read(&path).unwrap()).unwrap();
    // the column of the last write goes between planning and applying
    columns::delete_column(&scratch.connection, notes).unwrap();
    let (before, pupils_before) = (scratch.rows(class), scratch.pupil_count());
    let error = apply_import(&scratch.connection, class, &plan).unwrap_err();
    assert!(matches!(error, AppError::NotEditable), "{error:?}");
    assert_eq!(scratch.rows(class), before);
    assert_eq!(scratch.pupil_count(), pupils_before, "Eva, added first, is rolled back too");
}

#[test]
fn ods_files_import() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/import.ods");
    let summary = scratch.import(class, &golden);
    assert_eq!((summary.pupils, summary.new_pupils), (2, 2));
    let rows = scratch.rows(class);
    assert_eq!(rows[2], ("Uxía Núñez".to_owned(), strings(&["Bilingüe", "7.5", ""])), "accents intact");
    assert_eq!(rows[3], ("Brais Otero".to_owned(), strings(&["", "7.5", ""])), "LibreOffice read his 7,5 as a number");
}

/// LibreOffice's own formats keep dates, times and whole numbers their own way; each reads as the
/// same text as from .xlsx. Both files were saved once by LibreOffice and are never rewritten.
#[test]
fn ods_and_xls_dates_times_and_whole_numbers_import() {
    for fixture in ["import-types.ods", "import-types.xls"] {
        let scratch = Scratch::new();
        let (class, ..) = small_class(&scratch);
        for title in ["Born", "Time"] {
            scratch.column(class, ColumnType::Info, title);
        }
        scratch.import(class, &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(fixture));
        assert_eq!(scratch.rows(class)[0].1, strings(&["", "7", "", "2011-03-09", "1:30:05"]), "{fixture}");
    }
}

#[test]
fn the_table_lists_headers_and_named_rows() {
    let scratch = Scratch::new();
    let path = scratch
        .workbook("x.xlsx", &[vec![Text("Pupil"), Text("Exam"), Empty], vec![Text("Ana"), Number(1.5), Text("x")]]);
    let table = read(&path).unwrap();
    assert_eq!(table.headers, ["Exam", ""]);
    assert_eq!(table.rows[0].values, [Some("1.5".to_owned()), Some("x".to_owned())]);
}
