// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 10:48:30.959439
//+ Revised: 	2026/11/02 14:00:09.803770
// -----------------------------------------------------------------------------

//! The Excel export, read back with calamine as Excel would read it, and the import, from
//! workbooks written here with rust_xlsxwriter so each fixture reads in the diff.

mod common;

use std::path::{Path, PathBuf};

use calamine::{Data, Reader, Xlsx, open_workbook};
use rusqlite::Connection;
use tempfile::TempDir;

use avalio_lite::db;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::compute::{evaluate_row, parse_number};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{cells, classes, columns, pupils, sheet};
use avalio_lite::i18n::Language;
use avalio_lite::spreadsheet::export::{export_sheet, file_name, tab_name};
use avalio_lite::spreadsheet::import::{
    ImportRow, ImportSummary, ImportTable, LeftOut, apply_import, plan_import, read_table,
};
use avalio_lite::storage::partial_path;

/// A database in a scratch folder, where the exports go too.
struct Scratch {
    dir: TempDir,
    connection: Connection,
}

impl Scratch {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let connection = db::open(&dir.path().join("test.db"), None).unwrap();
        Self { dir, connection }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// Exports `class` in `language` and reads both tabs back.
    fn export(&self, class: i64, language: Language) -> Exported {
        let path = self.path("export.xlsx");
        export_sheet(&self.connection, class, language, &path).unwrap();
        Exported::read(&path)
    }

    fn column(&self, class: i64, kind: ColumnType, title: &str) -> i64 {
        let id = columns::add_column(&self.connection, class, kind, Language::English).unwrap();
        columns::rename_column(&self.connection, id, title).unwrap();
        id
    }
}

/// The tabs of an exported workbook, as cells.
struct Exported {
    names: Vec<String>,
    marks: Vec<Vec<Data>>,
    settings: Vec<Vec<Data>>,
}

impl Exported {
    fn read(path: &Path) -> Self {
        let mut workbook: Xlsx<_> = open_workbook(path).unwrap();
        let names = workbook.sheet_names();
        let mut tab = |name: &str| -> Vec<Vec<Data>> {
            workbook.worksheet_range(name).unwrap().rows().map(<[Data]>::to_vec).collect()
        };
        let (marks, settings) = (tab(&names[0]), tab(&names[1]));
        Self { names, marks, settings }
    }
}

fn text(value: &str) -> Data {
    Data::String(value.to_owned())
}

/// Class "1A": notes, an exam (60 %) and a lab (40 %, out of 5), a final grade, an average and
/// the final grade rounded to whole marks. Ana has every mark, Bea only the exam, Carla none.
fn full_class(scratch: &Scratch) -> i64 {
    let connection = &scratch.connection;
    let class = classes::create_class(connection, "1A").unwrap();
    let notes = scratch.column(class, ColumnType::Info, "Notes");
    let exam = scratch.column(class, ColumnType::Test, "Exam");
    columns::set_weight(connection, exam, "60").unwrap();
    let lab = scratch.column(class, ColumnType::Test, "Lab");
    columns::set_weight(connection, lab, "40").unwrap();
    columns::set_max_score(connection, lab, "5").unwrap();
    let final_grade = scratch.column(class, ColumnType::Weighted, "Final grade");
    scratch.column(class, ColumnType::Average, "Average");
    let rounded = scratch.column(class, ColumnType::Round, "Report card");
    columns::set_round_source(connection, rounded, Some(final_grade)).unwrap();
    columns::set_decimals(connection, rounded, "0").unwrap();
    let ana = pupils::add_new_pupil(connection, class, "Ana Pérez").unwrap();
    let bea = pupils::add_new_pupil(connection, class, "Bea").unwrap();
    pupils::add_new_pupil(connection, class, "Carla").unwrap();
    cells::set_cell(connection, notes, ana, "Moved from 1B").unwrap();
    cells::set_cell(connection, exam, ana, "7,5").unwrap();
    cells::set_cell(connection, lab, ana, "4").unwrap();
    cells::set_cell(connection, exam, bea, "5").unwrap();
    class
}

#[test]
fn exported_sheet_matches_the_screen() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let exported = scratch.export(class, Language::English);
    let loaded = sheet::load_sheet(&scratch.connection, class).unwrap();

    let header: Vec<Data> =
        std::iter::once(text("Pupil")).chain(loaded.columns.iter().map(|column| text(&column.title))).collect();
    assert_eq!(exported.marks[0], header);
    assert_eq!(exported.marks.len(), 1 + loaded.rows.len(), "no note: the weights add up");
    for (row, cells) in loaded.rows.iter().zip(&exported.marks[1..]) {
        assert_eq!(cells[0], text(&row.pupil.name));
        let shown = evaluate_row(&loaded.columns, &row.values);
        for ((column, shown), cell) in loaded.columns.iter().zip(&shown).zip(&cells[1..]) {
            let expected = match column.kind.column_type() {
                ColumnType::Info if shown.text.is_empty() => Data::Empty,
                ColumnType::Info => text(&shown.text),
                _ => parse_number(&shown.text).map_or(Data::Empty, Data::Float),
            };
            assert_eq!(*cell, expected, "{} of {}", column.title, row.pupil.name);
        }
    }
    // Ana: 7.5 × 60 % + 4/5 × 40 % = 7.7, the mean of 75 % and 80 % = 7.75, rounded 8
    assert_eq!(
        exported.marks[1],
        [
            text("Ana Pérez"),
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
        let pupil = pupils::add_new_pupil(connection, class, name).unwrap();
        cells::set_cell(connection, exam, pupil, mark).unwrap();
    }
    let marks: Vec<Data> =
        scratch.export(class, Language::English).marks[1..].iter().map(|row| row[1].clone()).collect();
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
    let lab = columns::list_columns(&scratch.connection, class).unwrap()[2].id;
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
    pupils::add_new_pupil(connection, class, "Ana").unwrap();
    assert_eq!(scratch.export(class, Language::English).marks.len(), 2, "like the screen, which shows no warning");
}

#[test]
fn empty_class_exports_the_header_only() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    scratch.column(class, ColumnType::Test, "Exam");
    let exported = scratch.export(class, Language::English);
    assert_eq!(exported.marks, [[text("Pupil"), text("Exam")]]);
}

#[test]
fn class_without_columns_exports_the_names() {
    let scratch = Scratch::new();
    let class = classes::create_class(&scratch.connection, "1A").unwrap();
    pupils::add_new_pupil(&scratch.connection, class, "Ana").unwrap();
    pupils::add_new_pupil(&scratch.connection, class, "Bea").unwrap();
    let exported = scratch.export(class, Language::English);
    assert_eq!(exported.marks, [[text("Pupil")], [text("Ana")], [text("Bea")]]);
    assert_eq!(exported.settings, [["Column", "Kind", "Weight %", "Maximum", "Bonus", "Rounds", "Decimals"].map(text)]);
}

#[test]
fn galician_labels_are_used() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let exported = scratch.export(class, Language::Galician);
    assert_eq!(exported.names, ["1A", "Columnas"]);
    assert_eq!(exported.marks[0][0], text("Alumno"));
    assert_eq!(
        exported.settings[0],
        ["Columna", "Tipo", "Peso %", "Máximo", "Extra", "Redondea", "Decimais"].map(text)
    );
    let kinds: Vec<Data> = exported.settings[1..].iter().map(|row| row[1].clone()).collect();
    assert_eq!(kinds, ["Información", "Proba", "Proba", "Nota final", "Media", "Redondeo"].map(text));
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
    let held = common::hold_exclusively(&path);
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
    let ana = pupils::add_new_pupil(connection, class, "Ana").unwrap();
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
// Import

/// A cell of a fixture workbook.
#[derive(Debug, Clone)]
enum Cell {
    Empty,
    Text(&'static str),
    Number(f64),
    Bool(bool),
    Date(u16, u8, u8),
    /// hours, minutes and seconds, shown as a duration (`[h]:mm:ss`)
    Duration(u16, u8, u8),
    /// a formula whose last result was this Excel error
    Error(&'static str),
}

use Cell::{Empty, Number, Text};

impl Scratch {
    /// Writes `rows` as the first tab of `name`, then a second tab the import must ignore.
    fn workbook(&self, name: &str, rows: &[Vec<Cell>]) -> PathBuf {
        use rust_xlsxwriter::{ExcelDateTime, Format, Formula, Workbook};
        let path = self.path(name);
        let mut workbook = Workbook::new();
        let tab = workbook.add_worksheet();
        let date = Format::new().set_num_format("yyyy-mm-dd");
        for (row, cells) in rows.iter().enumerate() {
            let row = u32::try_from(row).unwrap();
            for (column, cell) in cells.iter().enumerate() {
                let column = u16::try_from(column).unwrap();
                match cell {
                    Cell::Empty => {}
                    Cell::Text(text) => {
                        tab.write_string(row, column, *text).unwrap();
                    }
                    Cell::Number(number) => {
                        tab.write_number(row, column, *number).unwrap();
                    }
                    Cell::Bool(value) => {
                        tab.write_boolean(row, column, *value).unwrap();
                    }
                    Cell::Date(year, month, day) => {
                        let moment = ExcelDateTime::from_ymd(*year, *month, *day).unwrap();
                        tab.write_datetime_with_format(row, column, &moment, &date).unwrap();
                    }
                    Cell::Duration(hours, minutes, seconds) => {
                        let moment = ExcelDateTime::from_hms(*hours, *minutes, *seconds).unwrap();
                        let duration = Format::new().set_num_format("[h]:mm:ss");
                        tab.write_datetime_with_format(row, column, &moment, &duration).unwrap();
                    }
                    Cell::Error(error) => {
                        tab.write_formula(row, column, Formula::new("=1/0").set_result(*error)).unwrap();
                    }
                }
            }
        }
        workbook.add_worksheet().write_string(0, 0, "Ignored").unwrap();
        workbook.save(&path).unwrap();
        path
    }

    /// Reads, plans and applies `path` into `class`, as if the user said yes to replacing.
    fn import(&self, class: i64, path: &Path) -> ImportSummary {
        let table = read_table(path).unwrap();
        let plan = plan_import(&self.connection, class, &table).unwrap();
        apply_import(&self.connection, class, &plan).unwrap()
    }

    /// Each pupil of `class` with the values of their row, in sheet order.
    fn rows(&self, class: i64) -> Vec<(String, Vec<String>)> {
        let loaded = sheet::load_sheet(&self.connection, class).unwrap();
        loaded.rows.into_iter().map(|row| (row.pupil.name, row.values)).collect()
    }

    /// Each pupil of `class` with what each cell of their row shows on screen, numbers as numbers
    /// (a typed "7,5" and Excel's 7.5 are the same mark).
    fn shown(&self, class: i64) -> Vec<(String, Vec<String>)> {
        let loaded = sheet::load_sheet(&self.connection, class).unwrap();
        let as_number = |text: String| parse_number(&text).map_or(text, |number| number.to_string());
        let shown = |values: &[String]| {
            evaluate_row(&loaded.columns, values).into_iter().map(|cell| as_number(cell.text)).collect()
        };
        loaded.rows.iter().map(|row| (row.pupil.name.clone(), shown(&row.values))).collect()
    }

    fn pupil_count(&self) -> i64 {
        self.connection.query_row("SELECT COUNT(*) FROM pupils", [], |row| row.get(0)).unwrap()
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// Class "1A": notes, a test "Exam" and a final grade, with Ana and Bea and no marks.
fn small_class(scratch: &Scratch) -> (i64, i64, i64) {
    let connection = &scratch.connection;
    let class = classes::create_class(connection, "1A").unwrap();
    let notes = scratch.column(class, ColumnType::Info, "Notes");
    let exam = scratch.column(class, ColumnType::Test, "Exam");
    scratch.column(class, ColumnType::Weighted, "Final grade");
    pupils::add_new_pupil(connection, class, "Ana").unwrap();
    pupils::add_new_pupil(connection, class, "Bea").unwrap();
    (class, notes, exam)
}

fn pupil_ids(scratch: &Scratch, class: i64) -> Vec<i64> {
    pupils::list_class_pupils(&scratch.connection, class).unwrap().into_iter().map(|pupil| pupil.id).collect()
}

#[test]
fn an_exported_sheet_imports_back_unchanged() {
    let scratch = Scratch::new();
    let class = full_class(&scratch);
    let before = scratch.shown(class);
    let path = scratch.path("1A.xlsx");
    export_sheet(&scratch.connection, class, Language::English, &path).unwrap();
    for column in columns::list_columns(&scratch.connection, class).unwrap() {
        if !column.kind.is_computed() {
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

#[test]
fn pupils_match_in_class_then_elsewhere_then_new() {
    let scratch = Scratch::new();
    let connection = &scratch.connection;
    let (class, ..) = small_class(&scratch);
    let other = classes::create_class(connection, "2B").unwrap();
    let carla = pupils::add_new_pupil(connection, other, "Carla").unwrap();
    pupils::add_new_pupil(connection, other, "Dani").unwrap();
    let third = classes::create_class(connection, "3C").unwrap();
    pupils::add_new_pupil(connection, third, "dani").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam")],
            vec![Text(" ana "), Number(5.0)],
            vec![Text("CARLA"), Number(6.0)],
            vec![Text("Dani"), Number(7.0)],
            vec![Text("Eva"), Number(8.0)],
        ],
    );
    let pupils_before = scratch.pupil_count();
    let summary = scratch.import(class, &path);
    assert_eq!((summary.pupils, summary.new_pupils, summary.marks), (4, 2, 4));
    let names: Vec<String> = scratch.rows(class).into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["Ana", "Bea", "Carla", "Dani", "Eva"], "new pupils at the end, in file order");
    assert_eq!(pupil_ids(&scratch, class)[2], carla, "the one Carla elsewhere joins the class");
    assert_eq!(scratch.pupil_count(), pupils_before + 2, "two Danis elsewhere: which one is unknown, so a new one");
    assert_eq!(scratch.rows(class)[0].1[1], "5", "Ana matched ignoring case and spaces");
}

#[test]
fn a_name_twice_in_the_file_is_refused_and_changes_nothing() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam")],
            vec![Text("Ana Pérez"), Number(5.0)],
            vec![Text("ana pérez"), Number(6.0)],
        ],
    );
    let before = scratch.rows(class);
    let error = plan_import(&scratch.connection, class, &read_table(&path).unwrap()).unwrap_err();
    assert!(matches!(&error, AppError::DuplicatePupilInFile(name) if name == "ana pérez"), "{error:?}");
    assert_eq!(scratch.rows(class), before);
}

#[test]
fn empty_file_cells_keep_existing_values() {
    let scratch = Scratch::new();
    let (class, notes, exam) = small_class(&scratch);
    let ana = pupil_ids(&scratch, class)[0];
    cells::set_cell(&scratch.connection, notes, ana, "Keep me").unwrap();
    cells::set_cell(&scratch.connection, exam, ana, "3").unwrap();
    let path = scratch
        .workbook("x.xlsx", &[vec![Text("Pupil"), Text("Notes"), Text("Exam")], vec![Text("Ana"), Empty, Number(9.0)]]);
    let plan = plan_import(&scratch.connection, class, &read_table(&path).unwrap()).unwrap();
    assert_eq!(plan.overwrites, 1, "only the exam is replaced");
    apply_import(&scratch.connection, class, &plan).unwrap();
    assert_eq!(scratch.rows(class)[0].1, strings(&["Keep me", "9", ""]));
}

#[test]
fn unchanged_values_are_neither_counted_nor_asked_about() {
    let scratch = Scratch::new();
    let (class, _, exam) = small_class(&scratch);
    let ana = pupil_ids(&scratch, class)[0];
    cells::set_cell(&scratch.connection, exam, ana, "7,5").unwrap();
    let path = scratch.workbook("x.xlsx", &[vec![Text("Pupil"), Text("Exam")], vec![Text("Ana"), Number(7.5)]]);
    let plan = plan_import(&scratch.connection, class, &read_table(&path).unwrap()).unwrap();
    assert_eq!((plan.overwrites, plan.summary.marks), (0, 0), "7,5 and 7.5 are the same score");
    apply_import(&scratch.connection, class, &plan).unwrap();
    assert_eq!(scratch.rows(class)[0].1[1], "7,5", "kept as the user typed it");
}

#[test]
fn the_same_text_again_is_not_a_change() {
    let scratch = Scratch::new();
    let (class, notes, exam) = small_class(&scratch);
    let ana = pupil_ids(&scratch, class)[0];
    cells::set_cell(&scratch.connection, notes, ana, "Bilingüe").unwrap();
    cells::set_cell(&scratch.connection, exam, ana, "abc").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("Pupil"), Text("Notes"), Text("Exam")], vec![Text("Ana"), Text(" Bilingüe "), Text("abc")]],
    );
    let plan = plan_import(&scratch.connection, class, &read_table(&path).unwrap()).unwrap();
    assert_eq!(
        (plan.overwrites, plan.summary.marks),
        (0, 0),
        "re-importing what is already there neither asks nor counts, for notes and for unreadable scores"
    );
}

#[test]
fn excel_numbers_dates_and_booleans_become_text() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    for title in ["Born", "Repeats", "Count", "Time"] {
        scratch.column(class, ColumnType::Info, title);
    }
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![
                Text("Pupil"),
                Text("Notes"),
                Text("Exam"),
                Text("Born"),
                Text("Repeats"),
                Text("Count"),
                Text("Time"),
            ],
            vec![
                Text("Ana"),
                Text("  spaced  "),
                Number(7.125),
                Cell::Date(2011, 3, 9),
                Cell::Bool(true),
                Number(3.0),
                Cell::Duration(1, 30, 5),
            ],
            vec![Text("Bea"), Empty, Text("7,5")],
        ],
    );
    scratch.import(class, &path);
    assert_eq!(scratch.rows(class)[0].1, strings(&["spaced", "7.125", "", "2011-03-09", "TRUE", "3", "1:30:05"]));
    assert_eq!(scratch.rows(class)[1].1[1], "7,5", "a decimal comma typed as text is kept as typed");
}

#[test]
fn error_cells_are_counted_and_skipped() {
    let scratch = Scratch::new();
    let (class, _, exam) = small_class(&scratch);
    let ana = pupil_ids(&scratch, class)[0];
    cells::set_cell(&scratch.connection, exam, ana, "6").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam")],
            vec![Text("Ana"), Cell::Error("#DIV/0!")],
            vec![Text("Bea"), Cell::Error("#N/A")],
        ],
    );
    let summary = scratch.import(class, &path);
    assert_eq!((summary.error_cells, summary.marks), (2, 0));
    assert_eq!(scratch.rows(class)[0].1[1], "6", "an error cell counts as empty: the mark stays");
}

#[test]
fn invalid_scores_are_stored_like_typing() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook("x.xlsx", &[vec![Text("Pupil"), Text("Exam")], vec![Text("Ana"), Number(12.0)]]);
    scratch.import(class, &path);
    let loaded = sheet::load_sheet(&scratch.connection, class).unwrap();
    assert_eq!(loaded.rows[0].values[1], "12");
    assert!(evaluate_row(&loaded.columns, &loaded.rows[0].values)[1].is_invalid, "shaded red, as if typed");
}

#[test]
fn rows_without_a_name_are_skipped() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam")],
            vec![Empty, Number(3.0)],
            vec![Text("   "), Number(4.0)],
            vec![Text("Bea"), Number(5.0)],
        ],
    );
    let summary = scratch.import(class, &path);
    assert_eq!((summary.pupils, summary.marks), (1, 1));
}

#[test]
fn a_header_only_file_has_no_pupils() {
    let scratch = Scratch::new();
    let path = scratch.workbook("x.xlsx", &[vec![Text("Pupil"), Text("Exam")]]);
    let error = read_table(&path).unwrap_err();
    assert!(matches!(&error, AppError::NoPupilsInFile(failed) if *failed == path), "{error:?}");
    let empty = scratch.workbook("empty.xlsx", &[]);
    assert!(matches!(read_table(&empty).unwrap_err(), AppError::NoPupilsInFile(_)));
}

#[test]
fn a_text_file_renamed_xlsx_is_a_read_error() {
    let scratch = Scratch::new();
    let path = scratch.path("notes.xlsx");
    std::fs::write(&path, "just some notes").unwrap();
    let error = read_table(&path).unwrap_err();
    assert!(matches!(&error, AppError::SpreadsheetRead { path: failed, .. } if *failed == path), "{error:?}");
    assert!(matches!(read_table(&scratch.path("gone.xlsx")).unwrap_err(), AppError::SpreadsheetRead { .. }));
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
    let plan = plan_import(&scratch.connection, class, &read_table(&path).unwrap()).unwrap();
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
    let table = read_table(&path).unwrap();
    assert_eq!(table.headers, ["Exam", ""]);
    assert_eq!(table.rows[0].values, [Some("1.5".to_owned()), Some("x".to_owned())]);
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(32))]
    /// Random tables against a class with marks: the plan's counts are what applying changes.
    #[test]
    fn plan_counts_match_what_apply_writes(
        rows in proptest::collection::vec(
            ("Ana|bea|[A-Ea-e]{1,2}", proptest::option::of("[0-9]{1,2}|[a-z]{1,3}"), proptest::option::of("[a-z]{1,3}")),
            1..8,
        )
    ) {
        let scratch = Scratch::new();
        let (class, notes, exam) = small_class(&scratch);
        let ana = pupil_ids(&scratch, class)[0];
        cells::set_cell(&scratch.connection, exam, ana, "5").unwrap();
        cells::set_cell(&scratch.connection, notes, ana, "a").unwrap();
        let mut seen = std::collections::HashSet::new();
        let table = ImportTable {
            headers: strings(&["Exam", "Notes"]),
            rows: rows
                .into_iter()
                .filter(|(name, ..)| seen.insert(name.to_lowercase()))
                .map(|(name, exam, notes)| ImportRow { name, values: vec![exam, notes] })
                .collect(),
            error_cells: 0,
        };
        let (before, pupils_before) = (scratch.rows(class), scratch.pupil_count());
        let plan = plan_import(&scratch.connection, class, &table).unwrap();
        let summary = apply_import(&scratch.connection, class, &plan).unwrap();
        let changed: usize = scratch
            .rows(class)
            .iter()
            .map(|(name, values)| {
                let old = before.iter().find(|(old_name, _)| old_name == name).map(|(_, old)| old.clone());
                let old = old.unwrap_or_else(|| vec![String::new(); values.len()]);
                values.iter().zip(&old).filter(|(new, old)| new != old).count()
            })
            .sum();
        proptest::prop_assert_eq!(summary.marks, changed);
        proptest::prop_assert_eq!(scratch.pupil_count() - pupils_before, i64::try_from(summary.new_pupils).unwrap());
        proptest::prop_assert_eq!(summary.pupils, table.rows.len());
    }
}

mod cell_values {
    use avalio_lite::spreadsheet::import::{cell_text, clock, iso_seconds, without_float_noise};
    use calamine::{Data, ExcelDateTime, ExcelDateTimeType};
    use proptest::prelude::*;

    fn moment(value: f64, kind: ExcelDateTimeType) -> Data {
        Data::DateTime(ExcelDateTime::new(value, kind, false))
    }

    #[test]
    fn regression_a_time_of_day_is_a_time_not_a_date() {
        // an 08:30 cell (no date) imported as "1899-12-31"
        assert_eq!(cell_text(&moment(8.5 / 24.0, ExcelDateTimeType::DateTime)).as_deref(), Some("8:30:00"));
        assert_eq!(cell_text(&moment(45_000.25, ExcelDateTimeType::DateTime)).as_deref(), Some("2023-03-15"));
        assert_eq!(cell_text(&moment(1.5, ExcelDateTimeType::TimeDelta)).as_deref(), Some("36:00:00"));
    }

    #[test]
    fn regression_negative_durations_carry_one_sign() {
        // -5405 s showed as "-1:-30:-05"
        assert_eq!(clock(-5405.0), "-1:30:05");
        assert_eq!(clock(5405.0), "1:30:05");
        assert_eq!(clock(0.0), "0:00:00");
    }

    #[test]
    fn regression_formula_results_lose_their_float_noise() {
        // 7.3 worked out by a formula is 7.2999999999999989, which imported as "7.299999999999999"
        assert_eq!(cell_text(&Data::Float(7.3_f64.next_down())).as_deref(), Some("7.3"));
        assert_eq!(cell_text(&Data::Float(0.1 + 0.2)).as_deref(), Some("0.3"));
        assert_eq!(cell_text(&Data::Float(f64::NAN)), None);
        assert_eq!(cell_text(&Data::Float(f64::INFINITY)), None);
    }

    #[test]
    fn iso_durations_and_their_fallback() {
        assert_eq!(cell_text(&Data::DurationIso("PT01H30M05S".to_owned())).as_deref(), Some("1:30:05"));
        assert_eq!(cell_text(&Data::DurationIso("P1D".to_owned())).as_deref(), Some("P1D"), "kept as written");
        assert_eq!(cell_text(&Data::DateTimeIso("2011-03-09T08:30:00".to_owned())).as_deref(), Some("2011-03-09"));
        assert_eq!(cell_text(&Data::String("   ".to_owned())), None, "spaces only is empty");
    }

    proptest! {
        #[test]
        fn clock_reads_back(seconds in -1_000_000i64..1_000_000) {
            let text = clock(seconds as f64);
            let (sign, rest) = text.strip_prefix('-').map_or((1, text.as_str()), |rest| (-1, rest));
            let parts: Vec<i64> = rest.split(':').map(|part| part.parse().unwrap()).collect();
            prop_assert_eq!(sign * (parts[0] * 3600 + parts[1] * 60 + parts[2]), seconds);
            prop_assert!(parts[1] < 60 && parts[2] < 60);
        }

        #[test]
        fn iso_durations_read_as_seconds(hours in 0u32..1000, minutes in 0u32..60, seconds in 0u32..60) {
            let text = format!("PT{hours:02}H{minutes:02}M{seconds:02}S");
            prop_assert_eq!(iso_seconds(&text), Some(f64::from(hours * 3600 + minutes * 60 + seconds)));
        }

        #[test]
        fn cell_text_is_never_empty_or_padded(text in ".*") {
            if let Some(shown) = cell_text(&Data::String(text)) {
                prop_assert!(!shown.is_empty());
                prop_assert_eq!(shown.trim(), shown.as_str());
            }
        }

        #[test]
        fn short_decimals_import_as_typed(whole in 0u32..100_000, hundredths in 0u32..100) {
            let typed = f64::from(whole) + f64::from(hundredths) / 100.0;
            prop_assert_eq!(cell_text(&Data::Float(typed)), Some(typed.to_string()));
            prop_assert_eq!(without_float_noise(typed), typed);
        }
    }
}

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
