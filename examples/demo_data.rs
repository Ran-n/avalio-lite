// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:55.331624
//+ Revised: 	2026/10/28 10:52:47.072768
// -----------------------------------------------------------------------------

//! Creates a database with sample classes, pupils and marks, for trying the app or taking screenshots.
//! Usage: cargo run --example demo_data -- <path\to\avalio-lite.db> [en|gl]

use std::path::PathBuf;
use std::process::ExitCode;

use avalio_lite::db;
use avalio_lite::error::Result;
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{cells, classes, columns, pupils};
use avalio_lite::i18n::Language;
use rusqlite::Connection;

const PUPILS: [&str; 5] = ["Ana Pérez", "Brais Otero", "Carla Iglesias", "Daniel Souto", "Uxía Fernández"];
const MARKS: [[&str; 3]; 5] =
    [["8", "4,5", "9"], ["6,5", "3", "7"], ["9,5", "5", "10"], ["4", "2,5", ""], ["7", "4", "8,5"]];

/// Marks for the bonus test, out of 1.
const EXTRA: [&str; 5] = ["1", "", "1", "0,5", "1"];

/// The sample's own words (column titles and notes) in one language.
struct Words {
    exam: &'static str,
    lab: &'static str,
    project: &'static str,
    extra: &'static str,
    notes: [&'static str; 5],
}

fn words(language: Language) -> Words {
    match language {
        Language::English => Words {
            exam: "Exam 1",
            lab: "Lab report",
            project: "Project",
            extra: "Extra task",
            notes: ["Very participative", "", "Class delegate", "Missed the lab", ""],
        },
        Language::Galician => Words {
            exam: "Exame 1",
            lab: "Informe de laboratorio",
            project: "Proxecto",
            extra: "Tarefa extra",
            notes: ["Moi participativa", "", "Delegada de clase", "Faltou ao laboratorio", ""],
        },
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next().map(PathBuf::from) else {
        eprintln!("usage: demo_data <path to new .db file> [en|gl]");
        return ExitCode::FAILURE;
    };
    let Some(language) = Language::from_code(&args.next().unwrap_or_else(|| "en".to_owned())) else {
        eprintln!("unknown language; use en or gl");
        return ExitCode::FAILURE;
    };
    if path.exists() {
        eprintln!("{} already exists; not overwriting it", path.display());
        return ExitCode::FAILURE;
    }
    match db::open(&path, None).and_then(|connection| fill(&connection, language)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("could not create demo data: {error}");
            ExitCode::FAILURE
        }
    }
}

fn fill(connection: &Connection, language: Language) -> Result<()> {
    let words = words(language);
    let class_id = classes::create_class(connection, "2º ESO A")?;
    let other = classes::create_class(connection, "3º ESO B")?;
    let exam = columns::add_column(connection, class_id, ColumnType::Test, language)?;
    columns::rename_column(connection, exam, words.exam)?;
    columns::set_weight(connection, exam, "50")?;
    let lab = columns::add_column(connection, class_id, ColumnType::Test, language)?;
    columns::rename_column(connection, lab, words.lab)?;
    columns::set_weight(connection, lab, "20")?;
    columns::set_max_score(connection, lab, "5")?;
    let project = columns::add_column(connection, class_id, ColumnType::Test, language)?;
    columns::rename_column(connection, project, words.project)?;
    let extra = columns::add_column(connection, class_id, ColumnType::Test, language)?;
    columns::rename_column(connection, extra, words.extra)?;
    columns::set_bonus(connection, extra, true)?;
    columns::set_weight(connection, extra, "10")?;
    columns::set_max_score(connection, extra, "1")?;
    let notes = columns::add_column(connection, class_id, ColumnType::Info, language)?;
    columns::add_column(connection, class_id, ColumnType::Weighted, language)?;
    columns::add_column(connection, class_id, ColumnType::Round, language)?;
    for (((name, marks), note), bonus) in PUPILS.into_iter().zip(MARKS).zip(words.notes).zip(EXTRA) {
        let pupil = pupils::add_new_pupil(connection, class_id, name)?;
        for (column, value) in [exam, lab, project].into_iter().zip(marks) {
            cells::set_cell(connection, column, pupil, value)?;
        }
        cells::set_cell(connection, notes, pupil, note)?;
        cells::set_cell(connection, extra, pupil, bonus)?;
    }
    pupils::add_new_pupil(connection, other, "Xoán Rial")?;
    Ok(())
}
