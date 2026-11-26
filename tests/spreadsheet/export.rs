// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.342183
//+ Revised: 	2026/11/23 13:00:34.342183
// -----------------------------------------------------------------------------

//! Export: the sheet as written to a file, the columns tab, and the names and file names Excel refuses.

use crate::support::{Exported, Scratch, full_class, text};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::compute::{evaluate_row, parse_number};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{cells, classes, columns, pupils, sheet};
use avalio_lite::i18n::Language;
use avalio_lite::platform::storage::partial_path;
use avalio_lite::spreadsheet::export::{export_sheet, file_name, tab_name};
use calamine::Data;

#[test]
fn exported_sheet_matches_the_screen() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let exported = scratch.export(class, Language::English);
    let loaded = sheet::load_sheet(&scratch.connection, class).unwrap();

    let header: Vec<Data> = loaded
        .columns
        .iter()
        .map(|column| match column.kind.column_type() {
            ColumnType::FirstName => text("First name"),
            ColumnType::Surname => text("Surname"),
            _ => text(&column.title),
        })
        .collect();
    assert_eq!(exported.marks[0], header);
    assert_eq!(exported.marks.len(), 1 + loaded.rows.len(), "no note: the weights add up");
    for (row, cells) in loaded.rows.iter().zip(&exported.marks[1..]) {
        let shown = evaluate_row(&loaded.columns, &row.values);
        for ((column, shown), cell) in loaded.columns.iter().zip(&shown).zip(cells) {
            let expected = match column.kind.column_type() {
                ColumnType::FirstName | ColumnType::Surname | ColumnType::Info if shown.text.is_empty() => Data::Empty,
                ColumnType::FirstName | ColumnType::Surname | ColumnType::Info => text(&shown.text),
                _ => parse_number(&shown.text).map_or(Data::Empty, Data::Float),
            };
            assert_eq!(*cell, expected, "{} of {}", column.title, row.pupil.full_name());
        }
    }
    // Ana: 7.5 × 60 % + 4/5 × 40 % = 7.7, the mean of 75 % and 80 % = 7.75, rounded 8
    assert_eq!(
        exported.marks[1],
        [
            text("Ana"),
            text("Pérez López"),
            text("Moved from 1B"),
            Data::Float(7.5),
            Data::Float(4.0),
            Data::Float(7.7),
            Data::Float(7.75),
            Data::Float(8.0)
        ]
    );
    // Carla has nothing to show
    assert!(exported.marks[3][1..].iter().all(|cell| *cell == Data::Empty), "{:?}", exported.marks[3]);
}

#[test]
fn invalid_scores_are_kept_as_text() {
    let scratch = Scratch::new();
    let connection = &scratch.connection;
    let class = classes::create_class(connection, "1A").unwrap();
    let exam = scratch.column(class, ColumnType::Test, "Exam");
    for (name, mark) in [("Ana", "12"), ("Bea", "abc"), ("Carla", "9,25")] {
        let pupil = pupils::add_new_pupil(connection, class, name, "").unwrap();
        cells::set_cell(connection, exam, pupil, mark).unwrap();
    }
    let marks: Vec<Data> =
        scratch.export(class, Language::English).marks[1..].iter().map(|row| row[2].clone()).collect();
    assert_eq!(marks, [text("12"), text("abc"), Data::Float(9.25)]);
}

#[test]
fn the_columns_tab_lists_every_setting() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let project = scratch.column(class, ColumnType::Test, "Project");
    columns::set_weight(&scratch.connection, project, "10").unwrap();
    columns::set_bonus(&scratch.connection, project, true).unwrap();
    let unset = scratch.column(class, ColumnType::Round, "Not set up");
    columns::set_round_source(&scratch.connection, unset, None).unwrap();
    let exported = scratch.export(class, Language::English);
    let row = |title: &str, kind: &str, rest: [Data; 5]| {
        let mut cells = vec![text(title), text(kind)];
        cells.extend(rest);
        cells
    };
    let empty = || Data::Empty;
    assert_eq!(exported.names[1], "Columns");
    assert_eq!(
        exported.settings,
        [
            ["Column", "Kind", "Weight %", "Maximum", "Bonus", "Rounds", "Decimals"].map(text).to_vec(),
            row("First name", "First name", [empty(), empty(), empty(), empty(), empty()]),
            row("Surname", "Surname", [empty(), empty(), empty(), empty(), empty()]),
            row("Notes", "Info", [empty(), empty(), empty(), empty(), empty()]),
            row("Exam", "Test", [Data::Float(60.0), Data::Float(10.0), empty(), empty(), empty()]),
            row("Lab", "Test", [Data::Float(40.0), Data::Float(5.0), empty(), empty(), empty()]),
            row("Final grade", "Final grade", [empty(), Data::Float(10.0), empty(), empty(), empty()]),
            row("Average", "Average", [empty(), Data::Float(10.0), empty(), empty(), empty()]),
            row("Report card", "Rounded", [empty(), empty(), empty(), text("Final grade"), Data::Float(0.0)]),
            row("Project", "Test", [Data::Float(10.0), Data::Float(10.0), text("Yes"), empty(), empty()]),
            row("Not set up", "Rounded", [empty(), empty(), empty(), empty(), Data::Float(0.0)]),
        ]
    );
}

#[test]
fn incomplete_weights_add_the_provisional_note() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let lab = columns::list_columns(&scratch.connection, class).unwrap()[4].id;
    columns::set_weight(&scratch.connection, lab, "30").unwrap();
    let marks = scratch.export(class, Language::English).marks;
    assert_eq!(marks.len(), 1 + 3 + 2, "the note sits two rows below the last pupil");
    assert!(marks[4].iter().all(|cell| *cell == Data::Empty));
    assert_eq!(marks[5][0], text("Test weights add up to 90%, not 100%: final grades are provisional."));

    columns::set_weight(&scratch.connection, lab, "32,5").unwrap();
    let galician = scratch.export(class, Language::Galician).marks;
    assert_eq!(
        galician[5][0],
        text("Os pesos das probas suman 92,5 %, non 100 %: as notas finais son provisionais."),
        "the total in the language's own decimals"
    );

    columns::set_weight(&scratch.connection, lab, "40").unwrap();
    assert_eq!(scratch.export(class, Language::English).marks.len(), 1 + 3, "at 100 % there is no note");
}

#[test]
fn incomplete_weights_without_a_final_grade_need_no_note() {
    let scratch = Scratch::new();
    let connection = &scratch.connection;
    let class = classes::create_class(connection, "1A").unwrap();
    scratch.column(class, ColumnType::Test, "Exam");
    pupils::add_new_pupil(connection, class, "Ana", "").unwrap();
    assert_eq!(scratch.export(class, Language::English).marks.len(), 2, "like the screen, which shows no warning");
}

#[test]
fn empty_class_exports_the_header_only() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    scratch.column(class, ColumnType::Test, "Exam");
    let exported = scratch.export(class, Language::English);
    assert_eq!(exported.marks, [[text("First name"), text("Surname"), text("Exam")]]);
}

#[test]
fn class_without_columns_exports_the_names() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    pupils::add_new_pupil(&scratch.connection, class, "Ana", "").unwrap();
    pupils::add_new_pupil(&scratch.connection, class, "Bea", "").unwrap();
    let exported = scratch.export(class, Language::English);
    assert_eq!(
        exported.marks,
        [[text("First name"), text("Surname")], [text("Ana"), Data::Empty], [text("Bea"), Data::Empty]]
    );
    let names_only = |name: &str| {
        let mut row = vec![text(name), text(name)];
        row.extend(std::iter::repeat_n(Data::Empty, 5));
        row
    };
    assert_eq!(
        exported.settings,
        [
            ["Column", "Kind", "Weight %", "Maximum", "Bonus", "Rounds", "Decimals"].map(text).to_vec(),
            names_only("First name"),
            names_only("Surname"),
        ]
    );
}

#[test]
fn galician_labels_are_used() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let exported = scratch.export(class, Language::Galician);
    assert_eq!(exported.names, ["1A", "Columnas"]);
    assert_eq!(exported.marks[0][..2], [text("Nome"), text("Apelidos")]);
    assert_eq!(
        exported.settings[0],
        ["Columna", "Tipo", "Peso %", "Máximo", "Extra", "Redondea", "Decimais"].map(text)
    );
    let kinds: Vec<Data> = exported.settings[1..].iter().map(|row| row[1].clone()).collect();
    assert_eq!(
        kinds,
        ["Nome", "Apelidos", "Información", "Proba", "Proba", "Nota final", "Media", "Redondeo"].map(text)
    );
}

#[test]
fn tab_names_excel_refuses_are_changed() {
    let scratch = Scratch::new();
    for (class_name, tab) in [
        ("2º ESO [A]: maths/physics?", "2º ESO -A-- maths-physics-"),
        ("A class name far longer than Excel allows", "A class name far longer than Ex"),
        ("'Quoted'", "-Quoted-"),
        ("History", "History (1)"),
        ("Columns", "Columns (1)"),
        ("columns", "columns (1)"),
    ] {
        let class = classes::create_class(&scratch.connection, class_name).unwrap();
        assert_eq!(scratch.export(class, Language::English).names, [tab, "Columns"], "{class_name}");
    }
}

#[test]
fn an_empty_tab_name_falls_back() {
    assert_eq!(tab_name("", "Folla"), "Folla");
    assert_eq!(tab_name("1A", "Folla"), "1A");
}

#[test]
fn file_names_windows_refuses_are_changed() {
    assert_eq!(file_name("2º ESO A"), "2º ESO A");
    assert_eq!(file_name(r#"1A: "maths" \ physics?"#), "1A- -maths- - physics-");
    assert_eq!(file_name("Notes..."), "Notes---");
    assert_eq!(file_name("con"), "con-");
    assert_eq!(file_name("COM1"), "COM1-");
    assert_eq!(file_name("Console"), "Console");
    // Windows takes a device name before any dot for the device too
    assert_eq!(file_name("NUL.x"), "NUL-.x");
    assert_eq!(file_name("aux .notes"), "aux- .notes");
    assert_eq!(file_name("Conan.x"), "Conan.x");
}

#[test]
fn an_export_replaces_the_file_there() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let path = scratch.path("export.xlsx");
    std::fs::write(&path, "an older file").unwrap();
    export_sheet(&scratch.connection, class, Language::English, &path).unwrap();
    assert_eq!(Exported::read(&path).names[0], "1A");
    assert!(!partial_path(&path).exists());
}

#[test]
fn a_locked_destination_is_reported() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let path = scratch.path("open in Excel.xlsx");
    std::fs::write(&path, "the workbook Excel has open").unwrap();
    let held = crate::common::hold_exclusively(&path);
    let error = export_sheet(&scratch.connection, class, Language::English, &path).unwrap_err();
    drop(held);
    assert!(matches!(&error, AppError::SpreadsheetWrite { path: failed, .. } if *failed == path), "{error:?}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "the workbook Excel has open", "the old file is untouched");
    assert!(!partial_path(&path).exists(), "no half-written file is left");
}

#[test]
fn a_missing_folder_is_reported() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let path = scratch.path("gone").join("1A.xlsx");
    let error = export_sheet(&scratch.connection, class, Language::English, &path).unwrap_err();
    assert!(matches!(error, AppError::SpreadsheetWrite { .. }), "{error:?}");
}

#[test]
fn a_cell_too_long_for_excel_is_reported() {
    let scratch = Scratch::new();
    let connection = &scratch.connection;
    let class = classes::create_class(connection, "1A").unwrap();
    let notes = scratch.column(class, ColumnType::Info, "Notes");
    let ana = pupils::add_new_pupil(connection, class, "Ana", "").unwrap();
    cells::set_cell(connection, notes, ana, &"x".repeat(40_000)).unwrap();
    let path = scratch.path("export.xlsx");
    let error = export_sheet(connection, class, Language::English, &path).unwrap_err();
    assert!(matches!(error, AppError::SpreadsheetWrite { .. }), "{error:?}");
    assert!(!path.exists());
}

proptest::proptest! {
    #[test]
    fn tab_and_file_names_are_cleaned(text in "\\PC*|.*") {
        let tab = tab_name(&text, "Sheet");
        let length = tab.encode_utf16().count();
        proptest::prop_assert!((1..=31).contains(&length), "{tab:?} is {length} long");
        proptest::prop_assert!(!tab.contains(['[', ']', ':', '*', '?', '/', '\\']), "{tab:?}");
        proptest::prop_assert!(!tab.starts_with('\'') && !tab.ends_with('\''), "{tab:?}");
        proptest::prop_assert!(!tab.eq_ignore_ascii_case("History"), "{tab:?}");
        let file = file_name(&text);
        proptest::prop_assert!(!file.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']), "{file:?}");
        proptest::prop_assert!(!file.chars().any(char::is_control), "{file:?}");
        proptest::prop_assert!(!file.ends_with(['.', ' ']), "{file:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// Name columns anywhere in the sheet

#[test]
fn the_export_follows_the_sheet_order_with_the_names_in_the_middle() {
    let scratch = Scratch::new();
    let connection = &scratch.connection;
    let class = classes::create_class(connection, "1A").unwrap();
    let exam = scratch.column(class, ColumnType::Test, "Exam");
    let ana = pupils::add_new_pupil(connection, class, "Ana", "Pérez").unwrap();
    cells::set_cell(connection, exam, ana, "7").unwrap();
    let listed = columns::list_columns(connection, class).unwrap();
    // Surname, Exam, First name
    columns::move_column_to(connection, listed[0].id, 2).unwrap();
    let exported = scratch.export(class, Language::Galician);
    assert_eq!(exported.marks[0], [text("Apelidos"), text("Exam"), text("Nome")]);
    assert_eq!(exported.marks[1], [text("Pérez"), Data::Float(7.0), text("Ana")]);
    let kinds: Vec<Data> = exported.settings[1..].iter().map(|row| row[1].clone()).collect();
    assert_eq!(kinds, [text("Apelidos"), text("Proba"), text("Nome")], "the columns tab lists the names too");
}
