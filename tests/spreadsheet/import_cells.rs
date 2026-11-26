// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.428709
//+ Revised: 	2026/11/23 13:00:34.428709
// -----------------------------------------------------------------------------

//! Import of cell values and headers: how each cell and each header is read and matched.

use crate::support::Cell::{Empty, Number, Text};
use crate::support::{Scratch, full_class, pupil_ids, small_class, strings};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{cells, columns};
use avalio_lite::i18n::Language;
use avalio_lite::spreadsheet::export::export_sheet;
use avalio_lite::spreadsheet::import::LeftOut;

#[test]
fn an_exported_sheet_imports_back_unchanged() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let before = scratch.shown(class);
    let path = scratch.path("1A.xlsx");
    export_sheet(&scratch.connection, class, Language::English, &path).unwrap();
    for column in columns::list_columns(&scratch.connection, class).unwrap() {
        if column.kind.holds_cells() {
            for pupil in pupil_ids(&scratch, class) {
                cells::set_cell(&scratch.connection, column.id, pupil, "").unwrap();
            }
        }
    }
    let summary = scratch.import(class, &path);
    assert_eq!(scratch.shown(class), before, "the sheet shows what it showed");
    assert_eq!(scratch.rows(class)[0].1[1], "7.5", "Ana's 7,5 comes back as Excel's number");
    assert_eq!((summary.pupils, summary.new_pupils, summary.marks), (3, 0, 4));
    let calculated = ["Final grade", "Average", "Report card"].map(|title| LeftOut::Header(title.to_owned()));
    assert_eq!(summary.left_out, calculated, "calculated columns are left out, and named");
}

#[test]
fn columns_match_by_title_ignoring_case_and_spaces() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("Name"), Text("  EXAM "), Text("notes")], vec![Text("Ana"), Number(7.0), Text("Good")]],
    );
    let summary = scratch.import(class, &path);
    assert!(summary.left_out.is_empty(), "{:?}", summary.left_out);
    assert_eq!(scratch.rows(class)[0], ("Ana".to_owned(), strings(&["Good", "7", ""])));
}

#[test]
fn calculated_and_unknown_headers_are_left_out_and_named() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Phone"), Text("Final grade"), Empty, Empty, Text("Exam")],
            vec![Text("Ana"), Text("600"), Number(9.0), Empty, Text("stray"), Number(6.0)],
        ],
    );
    let summary = scratch.import(class, &path);
    assert_eq!(
        summary.left_out,
        [LeftOut::Header("Phone".into()), LeftOut::Header("Final grade".into()), LeftOut::Untitled("E".into())],
        "an untitled column only counts when it has values, and is named by its letter"
    );
    assert_eq!(scratch.rows(class)[0].1, strings(&["", "6", ""]));
}

#[test]
fn a_repeated_header_uses_the_first() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("Pupil"), Text("Exam"), Text("exam")], vec![Text("Ana"), Number(4.0), Number(9.0)]],
    );
    let summary = scratch.import(class, &path);
    assert_eq!(summary.left_out, [LeftOut::Header("exam".into())]);
    assert_eq!(scratch.rows(class)[0].1[1], "4");
}

#[test]
fn a_header_fills_the_first_column_with_its_title() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    scratch.column(class, ColumnType::Test, "Exam");
    let path = scratch.workbook("x.xlsx", &[vec![Text("Pupil"), Text("Exam")], vec![Text("Ana"), Number(4.0)]]);
    scratch.import(class, &path);
    assert_eq!(scratch.rows(class)[0].1, strings(&["", "4", "", ""]));
}
