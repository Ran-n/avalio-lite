// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.644063
//+ Revised: 	2026/11/24 16:56:06.276214
// -----------------------------------------------------------------------------

//! Import of the name columns: surnames, first names and the name columns anywhere in the sheet.

use crate::support::Cell::{Empty, Number, Text};
use crate::support::{Cell, Scratch, read, small_class, strings};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{classes, pupils};
use avalio_lite::spreadsheet::import::{LeftOut, plan_import};

#[test]
fn a_column_left_out_twice_is_named_once() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Phone"), Text("Phone"), Text("Exam"), Text("EXAM")],
            vec![Text("Ana"), Text("600"), Text("700"), Number(4.0), Number(9.0)],
        ],
    );
    let summary = scratch.import(class, &path);
    assert_eq!(summary.left_out, [LeftOut::Header("Phone".into()), LeftOut::Header("EXAM".into())]);
}

// ---------------------------------------------------------------------------------------------
// First names and surnames

fn pupil_names(scratch: &Scratch, class: i64) -> Vec<(String, String)> {
    pupils::list_class_pupils(&scratch.connection, class)
        .unwrap()
        .into_iter()
        .map(|pupil| (pupil.first_name, pupil.surname))
        .collect()
}

fn named(first_name: &str, surname: &str) -> (String, String) {
    (first_name.to_owned(), surname.to_owned())
}

#[test]
fn a_one_name_column_file_matches_pupils_by_full_name() {
    let scratch = Scratch::new();
    let connection = &scratch.connection;
    let class = classes::create_class(connection, "1A").unwrap();
    scratch.column(class, ColumnType::Test, "Exam");
    pupils::add_new_pupil(connection, class, "Ana", "Pérez").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam")],
            vec![Text("ana PÉREZ"), Number(7.0)],
            vec![Text("Xoán Rial"), Number(5.0)],
        ],
    );
    let summary = scratch.import(class, &path);
    assert_eq!((summary.pupils, summary.new_pupils), (2, 1));
    assert_eq!(
        pupil_names(&scratch, class),
        [named("Ana", "Pérez"), named("Xoán Rial", "")],
        "the whole name of a new pupil is their first name"
    );
    assert_eq!(scratch.rows(class)[0].1, strings(&["7"]));
}

#[test]
fn a_two_name_column_file_makes_new_pupils_with_both_names() {
    let scratch = Scratch::new();
    let connection = &scratch.connection;
    let class = classes::create_class(connection, "1A").unwrap();
    scratch.column(class, ColumnType::Test, "Exam");
    // a pupil whose whole name is still in the first name is the same pupil
    pupils::add_new_pupil(connection, class, "Brais Otero", "").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("First name"), Text("Surname"), Text("Exam")],
            vec![Text("Ana"), Text("Pérez López"), Number(7.0)],
            vec![Text("Brais"), Text("Otero"), Number(6.0)],
            vec![Text("Carla"), Empty, Number(9.0)],
        ],
    );
    let table = read(&path).unwrap();
    assert_eq!((table.header_columns.clone(), table.headers.clone()), (vec![2], strings(&["Exam"])));
    let summary = scratch.import(class, &path);
    assert_eq!((summary.pupils, summary.new_pupils, summary.marks), (3, 2, 3));
    assert_eq!(
        pupil_names(&scratch, class),
        [named("Brais Otero", ""), named("Ana", "Pérez López"), named("Carla", "")]
    );
    assert_eq!(scratch.rows(class)[0].1, strings(&["6"]), "the marks start at column C");
}

#[test]
fn a_galician_surname_header_is_recognised_in_english() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    scratch.column(class, ColumnType::Test, "Exam");
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("Nome"), Text(" APELIDOS "), Text("Exam")], vec![Text("Ana"), Text("Pérez"), Number(7.0)]],
    );
    // whatever the interface language is: the reader knows every language's surname header
    scratch.import(class, &path);
    assert_eq!(pupil_names(&scratch, class), [named("Ana", "Pérez")]);
}

#[test]
fn a_row_with_a_surname_but_no_first_name_is_skipped() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("First name"), Text("Surname")], vec![Empty, Text("Pérez")], vec![Text("Bea"), Text("Otero")]],
    );
    let summary = scratch.import(class, &path);
    assert_eq!(summary.pupils, 1);
    assert_eq!(pupil_names(&scratch, class), [named("Bea", "Otero")]);
}

#[test]
fn a_comma_in_the_first_name_is_kept_when_the_file_has_a_surname_column() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    let path =
        scratch.workbook("x.xlsx", &[vec![Text("First name"), Text("Surname")], vec![Text("Otero, Bea"), Empty]]);
    scratch.import(class, &path);
    assert_eq!(pupil_names(&scratch, class), [named("Otero, Bea", "")], "only a lone name column is split");
}

#[test]
fn an_untitled_column_after_the_surname_is_named_column_d() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    scratch.column(class, ColumnType::Test, "Exam");
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("First name"), Text("Surname"), Text("Exam"), Empty],
            vec![Text("Ana"), Text("Pérez"), Number(7.0), Text("stray")],
        ],
    );
    let summary = scratch.import(class, &path);
    assert_eq!(summary.left_out, [LeftOut::Untitled("D".into())], "the letter counts both name columns");
}

#[test]
fn a_repeated_full_name_in_the_file_is_refused() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("First name"), Text("Surname")],
            vec![Text("Ana"), Text("Pérez")],
            vec![Text("ana"), Text("pérez")],
        ],
    );
    let error = plan_import(&scratch.connection, class, &read(&path).unwrap()).unwrap_err();
    assert!(matches!(&error, AppError::DuplicatePupilInFile(name) if name == "ana pérez"), "{error:?}");
}

#[test]
fn an_import_finds_the_name_columns_by_their_headers_anywhere() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    scratch.column(class, ColumnType::Test, "Exam");
    scratch.column(class, ColumnType::Info, "Notes");
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Exam"), Text("Notes"), Text("Apelidos"), Empty, Text("Nome")],
            vec![Number(7.0), Text("shy"), Text("Pérez"), Text("stray"), Text("Ana")],
        ],
    );
    let table = read(&path).unwrap();
    assert_eq!(table.header_columns, [0, 1, 3], "every column but the names");
    let summary = scratch.import(class, &path);
    assert_eq!(pupil_names(&scratch, class), [named("Ana", "Pérez")]);
    assert_eq!(scratch.rows(class)[0].1, strings(&["7", "shy"]));
    assert_eq!(summary.left_out, [LeftOut::Untitled("D".into())], "named by its own letter, names in between");
}

#[test]
fn a_surname_header_without_a_first_name_header_takes_the_first_name_from_column_a() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    scratch.column(class, ColumnType::Test, "Exam");
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("Pupil"), Text("Exam"), Text("Surname")], vec![Text("Ana"), Number(6.0), Text("Pérez")]],
    );
    scratch.import(class, &path);
    assert_eq!(pupil_names(&scratch, class), [named("Ana", "Pérez")]);
    assert_eq!(scratch.rows(class)[0].1, strings(&["6"]));
}

/// With no first-name header, the first name is in column A, unless column A is the surname: then
/// it is in column B.
#[test]
fn a_surname_in_column_a_makes_column_b_the_first_name() {
    let scratch = Scratch::new();
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("Surname"), Text("Who"), Text("Exam")], vec![Text("Pérez"), Text("Ana"), Number(7.0)]],
    );
    let table = read(&path).unwrap();
    assert_eq!(table.headers, ["Exam"], "the name columns are no marks");
    assert_eq!((table.rows[0].first_name.as_str(), table.rows[0].surname.as_str()), ("Ana", "Pérez"));
    assert_eq!(table.rows[0].values, [Some("7".to_owned())]);
}

#[test]
fn a_false_cell_becomes_the_text_false() {
    let scratch = Scratch::new();
    let path =
        scratch.workbook("x.xlsx", &[vec![Text("Pupil"), Text("Repeats")], vec![Text("Ana"), Cell::Bool(false)]]);
    assert_eq!(read(&path).unwrap().rows[0].values, [Some("FALSE".to_owned())]);
}
