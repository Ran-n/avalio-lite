// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.913603
//+ Revised: 	2026/11/23 13:00:34.913603
// -----------------------------------------------------------------------------

//! Helpers shared by the spreadsheet tests: the scratch folder, expected classes and cells.

use avalio_lite::gradebook::compute::{evaluate_row, parse_number};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{cells, classes, columns, pupils, sheet};
use avalio_lite::i18n::{self, Language};
use avalio_lite::platform::db;
use avalio_lite::spreadsheet::export::export_sheet;
use avalio_lite::spreadsheet::import::{
    ImportSummary, ImportTable, NameHeaders, apply_import, plan_import, read_table,
};
use calamine::{Data, Reader, Xlsx, open_workbook};
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A database in a scratch folder, where the exports go too.
pub(crate) struct Scratch {
    pub(crate) dir: TempDir,
    pub(crate) connection: Connection,
}

impl Scratch {
    pub(crate) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let connection = db::open(&dir.path().join("test.db"), None).unwrap();
        Self { dir, connection }
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// Exports `class` in `language` and reads both tabs back.
    pub(crate) fn export(&self, class: i64, language: Language) -> Exported {
        let path = self.path("export.xlsx");
        export_sheet(&self.connection, class, language, &path).unwrap();
        Exported::read(&path)
    }

    pub(crate) fn column(&self, class: i64, kind: ColumnType, title: &str) -> i64 {
        let id = columns::add_column(&self.connection, class, kind, Language::English).unwrap();
        columns::rename_column(&self.connection, id, title).unwrap();
        id
    }
}

/// The tabs of an exported workbook, as cells.
pub(crate) struct Exported {
    pub(crate) names: Vec<String>,
    pub(crate) marks: Vec<Vec<Data>>,
    pub(crate) settings: Vec<Vec<Data>>,
}

impl Exported {
    pub(crate) fn read(path: &Path) -> Self {
        let mut workbook: Xlsx<_> = open_workbook(path).unwrap();
        let names = workbook.sheet_names();
        let mut tab = |name: &str| -> Vec<Vec<Data>> {
            workbook.worksheet_range(name).unwrap().rows().map(<[Data]>::to_vec).collect()
        };
        let (marks, settings) = (tab(&names[0]), tab(&names[1]));
        Self { names, marks, settings }
    }
}

pub(crate) fn text(value: &str) -> Data {
    Data::String(value.to_owned())
}

/// Class "1A": notes, an exam (60 %) and a lab (40 %, out of 5), a final grade, an average and
/// the final grade rounded to whole marks. Ana Pérez López has every mark, Bea only the exam,
/// Carla none; only Ana has a surname.
pub(crate) fn full_class(scratch: &Scratch) -> i64 {
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
    let ana = pupils::add_new_pupil(connection, class, "Ana", "Pérez López").unwrap();
    let bea = pupils::add_new_pupil(connection, class, "Bea", "").unwrap();
    pupils::add_new_pupil(connection, class, "Carla", "").unwrap();
    cells::set_cell(connection, notes, ana, "Moved from 1B").unwrap();
    cells::set_cell(connection, exam, ana, "7,5").unwrap();
    cells::set_cell(connection, lab, ana, "4").unwrap();
    cells::set_cell(connection, exam, bea, "5").unwrap();
    class
}

// ---------------------------------------------------------------------------------------------
// Import

/// A cell of a fixture workbook.
#[derive(Debug, Clone)]
pub(crate) enum Cell {
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

/// Writes one `cell` at `(row, column)` of `tab`; dates in the `date` format.
pub(crate) fn write_cell(
    tab: &mut rust_xlsxwriter::Worksheet,
    (row, column): (u32, u16),
    cell: &Cell,
    date: &rust_xlsxwriter::Format,
) {
    use rust_xlsxwriter::{ExcelDateTime, Format, Formula};
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
            tab.write_datetime_with_format(row, column, &moment, date).unwrap();
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

impl Scratch {
    /// Writes `rows` as the first tab of `name`, then a second tab the import must ignore.
    pub(crate) fn workbook(&self, name: &str, rows: &[Vec<Cell>]) -> PathBuf {
        use rust_xlsxwriter::{Format, Workbook};
        let path = self.path(name);
        let mut workbook = Workbook::new();
        let tab = workbook.add_worksheet();
        let date = Format::new().set_num_format("yyyy-mm-dd");
        for (row, cells) in rows.iter().enumerate() {
            let row = u32::try_from(row).unwrap();
            for (column, cell) in cells.iter().enumerate() {
                write_cell(tab, (row, u16::try_from(column).unwrap()), cell, &date);
            }
        }
        workbook.add_worksheet().write_string(0, 0, "Ignored").unwrap();
        workbook.save(&path).unwrap();
        path
    }

    /// Reads, plans and applies `path` into `class`, as if the user said yes to replacing.
    pub(crate) fn import(&self, class: i64, path: &Path) -> ImportSummary {
        let table = read(path).unwrap();
        let plan = plan_import(&self.connection, class, &table).unwrap();
        apply_import(&self.connection, class, &plan).unwrap()
    }

    /// Each pupil of `class` with the values of their row in the mark columns, in sheet order.
    pub(crate) fn rows(&self, class: i64) -> Vec<(String, Vec<String>)> {
        let loaded = sheet::load_sheet(&self.connection, class).unwrap();
        loaded.rows.iter().map(|row| (row.pupil.full_name(), mark_values(&loaded.columns, &row.values))).collect()
    }

    /// Each pupil of `class` with what each cell of their row shows on screen, numbers as numbers
    /// (a typed "7,5" and Excel's 7.5 are the same mark).
    pub(crate) fn shown(&self, class: i64) -> Vec<(String, Vec<String>)> {
        let loaded = sheet::load_sheet(&self.connection, class).unwrap();
        let as_number = |text: String| parse_number(&text).map_or(text, |number| number.to_string());
        let shown = |values: &[String]| {
            let all: Vec<String> =
                evaluate_row(&loaded.columns, values).into_iter().map(|cell| as_number(cell.text)).collect();
            mark_values(&loaded.columns, &all)
        };
        loaded.rows.iter().map(|row| (row.pupil.full_name(), shown(&row.values))).collect()
    }

    pub(crate) fn pupil_count(&self) -> i64 {
        self.connection.query_row("SELECT COUNT(*) FROM pupils", [], |row| row.get(0)).unwrap()
    }
}

/// Reads a workbook as the app does: the name columns are found by the first-name and surname
/// headers of every language.
pub(crate) fn read(path: &Path) -> avalio_lite::error::Result<ImportTable> {
    let (first_name, surname): (Vec<&str>, Vec<&str>) =
        (i18n::first_name_headers().collect(), i18n::surname_headers().collect());
    read_table(path, &NameHeaders { first_name: &first_name, surname: &surname })
}

/// `values` without those of the first-name and surname columns.
pub(crate) fn mark_values(columns: &[avalio_lite::gradebook::model::Column], values: &[String]) -> Vec<String> {
    columns.iter().zip(values).filter(|(column, _)| !column.kind.is_name()).map(|(_, value)| value.clone()).collect()
}

pub(crate) fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// Class "1A": notes, a test "Exam" and a final grade, with Ana and Bea and no marks.
pub(crate) fn small_class(scratch: &Scratch) -> (i64, i64, i64) {
    let connection = &scratch.connection;
    let class = classes::create_class(connection, "1A").unwrap();
    let notes = scratch.column(class, ColumnType::Info, "Notes");
    let exam = scratch.column(class, ColumnType::Test, "Exam");
    scratch.column(class, ColumnType::Weighted, "Final grade");
    pupils::add_new_pupil(connection, class, "Ana", "").unwrap();
    pupils::add_new_pupil(connection, class, "Bea", "").unwrap();
    (class, notes, exam)
}

pub(crate) fn pupil_ids(scratch: &Scratch, class: i64) -> Vec<i64> {
    pupils::list_class_pupils(&scratch.connection, class).unwrap().into_iter().map(|pupil| pupil.id).collect()
}
