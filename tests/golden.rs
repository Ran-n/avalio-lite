// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:06.706242
//+ Revised: 	2026/11/11 17:11:01.788874
// -----------------------------------------------------------------------------

//! Files that outlive an app version, pinned as checked-in samples in `tests/golden/`: the settings
//! file and the database, in every format a user may still have, and the Excel export. Each is built from fixed data,
//! so the samples only change when the format does.
//!
//! `AVALIO_UPDATE_GOLDEN=1 cargo test --test golden` rewrites the samples of the *current* formats
//! after a deliberate format change (review the diff, then commit). Samples of older formats are
//! frozen: they stand for files already on users' disks, so they are only ever added, never rebuilt.

use std::fs;
use std::path::{Path, PathBuf};

use avalio_lite::error::AppError;
use avalio_lite::gradebook::model::{ColumnKind, ColumnType};
use avalio_lite::gradebook::{cells, classes, columns, pupils, sheet};
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::{AppConfig, ConfigStore, Preferences, ThemeChoice};
use avalio_lite::platform::db::{self, migrations};
use rusqlite::Connection;

const UPDATE_ENV: &str = "AVALIO_UPDATE_GOLDEN";
const PIN: &str = "2468";

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden").join(name)
}

fn updating() -> bool {
    std::env::var_os(UPDATE_ENV).is_some_and(|value| value == "1")
}

/// A scratch copy of a golden file, so opening (and migrating) it never touches the sample.
fn scratch_copy(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    fs::copy(golden(name), &path).unwrap_or_else(|error| panic!("missing golden file {name}: {error}"));
    (dir, path)
}

// ---------------------------------------------------------------------------------------------
// Settings file

fn current_config() -> AppConfig {
    AppConfig {
        database_path: PathBuf::from(r"D:\Marks\avalio-lite.db"),
        log_dir: PathBuf::from(r"D:\Marks\logs"),
        preferences: Preferences {
            show_column_settings: false,
            language: Some(Language::Galician),
            theme: Some(ThemeChoice::Dark),
            sidebar_collapsed: true,
            check_updates: false,
            last_update_check: chrono::DateTime::from_timestamp(1_800_000_000, 0),
            skipped_update: Some("0.6.0".to_owned()),
            last_run_version: Some("0.5.0".to_owned()),
            update_failed: Some("the installer couldn't start".to_owned()),
        },
        shortcuts: [("lock".to_owned(), "Ctrl+K".to_owned())].into(),
    }
}

#[test]
fn the_settings_file_is_written_exactly_as_the_golden_sample() {
    let dir = tempfile::tempdir().unwrap();
    let store = ConfigStore::in_dir(dir.path());
    store.save(&current_config()).unwrap();
    let written = fs::read_to_string(store.path()).unwrap();
    if updating() {
        fs::write(golden("config-current.toml"), &written).unwrap();
    }
    let expected = fs::read_to_string(golden("config-current.toml")).unwrap();
    assert_eq!(written, expected, "the settings format changed; see the module docs");
}

#[test]
fn the_golden_settings_file_loads() {
    let dir = tempfile::tempdir().unwrap();
    fs::copy(golden("config-current.toml"), dir.path().join("config.toml")).unwrap();
    assert_eq!(ConfigStore::in_dir(dir.path()).load().unwrap(), Some(current_config()));
}

#[test]
fn a_settings_file_from_before_updates_looks_for_them_by_itself() {
    let dir = tempfile::tempdir().unwrap();
    fs::copy(golden("config-v2.toml"), dir.path().join("config.toml")).unwrap();
    let preferences = ConfigStore::in_dir(dir.path()).load().unwrap().unwrap().preferences;
    assert_eq!(preferences.language, Some(Language::Galician), "what it had is kept");
    assert!(preferences.check_updates);
    assert_eq!((preferences.last_update_check, preferences.skipped_update), (None, None));
}

#[test]
fn a_settings_file_from_before_preferences_loads_with_the_defaults() {
    let dir = tempfile::tempdir().unwrap();
    fs::copy(golden("config-v1.toml"), dir.path().join("config.toml")).unwrap();
    let config = ConfigStore::in_dir(dir.path()).load().unwrap().unwrap();
    assert_eq!(config.database_path, Path::new(r"D:\Marks\avalio-lite.db"));
    assert_eq!(config.preferences, Preferences::default());
}

// ---------------------------------------------------------------------------------------------
// Database

/// Class "1A": Ana Pérez (7.5) and Bea (4) in "Exam" (100 %, out of 10), a final grade, and a note;
/// the notes, first-name and surname columns resized.
fn fill_current(connection: &Connection) {
    let class = classes::create_class(connection, "1A").unwrap();
    let ana = pupils::add_new_pupil(connection, class, "Ana", "Pérez").unwrap();
    let bea = pupils::add_new_pupil(connection, class, "Bea", "").unwrap();
    let exam = columns::add_column(connection, class, ColumnType::Test, Language::English).unwrap();
    columns::rename_column(connection, exam, "Exam").unwrap();
    let bonus = columns::add_column(connection, class, ColumnType::Test, Language::English).unwrap();
    columns::rename_column(connection, bonus, "Project").unwrap();
    columns::set_weight(connection, bonus, "10").unwrap();
    columns::set_bonus(connection, bonus, true).unwrap();
    columns::add_column(connection, class, ColumnType::Weighted, Language::English).unwrap();
    let notes = columns::add_column(connection, class, ColumnType::Info, Language::English).unwrap();
    cells::set_cell(connection, exam, ana, "7,5").unwrap();
    cells::set_cell(connection, exam, bea, "4").unwrap();
    cells::set_cell(connection, bonus, ana, "10").unwrap();
    cells::set_cell(connection, notes, bea, "Needs glasses").unwrap();
    columns::set_column_width(connection, notes, Some(300)).unwrap();
    let names = columns::list_columns(connection, class).unwrap();
    columns::set_column_width(connection, names[0].id, Some(260)).unwrap();
    columns::set_column_width(connection, names[1].id, Some(180)).unwrap();
    // the surname after the exam, as a teacher may move it
    columns::move_column_to(connection, names[1].id, 2).unwrap();
}

/// The column widths of the golden class, in sheet order (the first-name and surname columns
/// first, from samples before they could move): `None` follows the kind's default.
fn sheet_widths(connection: &Connection) -> Vec<Option<u16>> {
    let class = classes::list_classes(connection).unwrap()[0].id;
    sheet::load_sheet(connection, class).unwrap().columns.iter().map(|column| column.width).collect()
}

/// The kinds of the golden class's columns in sheet order, the names as F (first name) and S.
fn column_order(connection: &Connection) -> String {
    let class = classes::list_classes(connection).unwrap()[0].id;
    let loaded = sheet::load_sheet(connection, class).unwrap();
    loaded
        .columns
        .iter()
        .map(|column| match column.kind {
            ColumnKind::FirstName => 'F',
            ColumnKind::Surname => 'S',
            _ => 'm',
        })
        .collect()
}

/// What the golden databases show for class "1A", whatever their version: the mark columns, the
/// pupils and their marks.
fn assert_current_sheet(connection: &Connection) {
    let class = classes::list_classes(connection).unwrap()[0].id;
    let loaded = sheet::load_sheet(connection, class).unwrap();
    let marks: Vec<usize> = (0..loaded.columns.len()).filter(|index| !loaded.columns[*index].kind.is_name()).collect();
    let titles: Vec<&str> = marks.iter().map(|index| loaded.columns[*index].title.as_str()).collect();
    assert_eq!(titles, ["Exam", "Project", "Final grade", "Notes"]);
    let names: Vec<&str> = loaded.rows.iter().map(|row| row.pupil.first_name.as_str()).collect();
    assert_eq!(names, ["Ana", "Bea"]);
    let values =
        |row: usize| -> Vec<&str> { marks.iter().map(|index| loaded.rows[row].values[*index].as_str()).collect() };
    assert_eq!(values(0), ["7,5", "10", "", ""]);
    assert_eq!(values(1), ["4", "", "", "Needs glasses"]);
}

/// Rebuilds a current-format database sample from scratch.
fn write_current_database(name: &str, key: Option<&str>) {
    let path = golden(name);
    db::remove_files(&path).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let source = db::open(&dir.path().join("source.db"), None).unwrap();
    fill_current(&source);
    // a single self-contained file: what an export (move, re-key) writes
    db::export(&source, &path, key).unwrap();
}

#[test]
fn the_current_database_sample_opens_with_its_data() {
    if updating() {
        write_current_database("db-current.db", None);
    }
    let (_dir, path) = scratch_copy("db-current.db");
    assert!(!db::is_encrypted(&path).unwrap());
    let connection = db::open(&path, None).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
    assert_current_sheet(&connection);
    assert_eq!(column_order(&connection), "FmSmmm", "the surname was moved after the exam");
    assert_eq!(sheet_widths(&connection), [Some(260), None, Some(180), None, None, Some(300)]);
    let class = classes::list_classes(&connection).unwrap()[0].id;
    assert_eq!(pupils::list_class_pupils(&connection, class).unwrap()[0].surname, "Pérez");
}

#[test]
fn the_encrypted_database_sample_opens_with_its_pin_only() {
    if updating() {
        write_current_database("db-current-encrypted.db", Some(PIN));
    }
    let (_dir, path) = scratch_copy("db-current-encrypted.db");
    assert!(db::is_encrypted(&path).unwrap());
    assert!(matches!(db::open_existing(&path, None), Err(AppError::WrongPin)));
    assert!(matches!(db::open_existing(&path, Some("1357")), Err(AppError::WrongPin)));
    let connection = db::open_existing(&path, Some(PIN)).unwrap();
    assert_current_sheet(&connection);
}

#[test]
fn a_version_1_database_upgrades_with_all_its_data() {
    let (_dir, path) = scratch_copy("db-v1.db");
    let old = Connection::open(&path).unwrap();
    assert_eq!(migrations::schema_version(&old).unwrap(), 1, "the sample really is version 1");
    drop(old);
    let connection = db::open(&path, None).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
    let class = classes::list_classes(&connection).unwrap()[0].id;
    let loaded = sheet::load_sheet(&connection, class).unwrap();
    let titles: Vec<&str> =
        loaded.columns.iter().filter(|column| !column.kind.is_name()).map(|column| column.title.as_str()).collect();
    assert_eq!(titles, ["Exam", "Final grade"]);
    assert_eq!(loaded.rows[0].pupil.first_name, "Ana");
    assert_eq!(loaded.rows[0].values, ["Ana", "", "7,5", ""], "the names first, then the marks");
    assert!(
        loaded
            .columns
            .iter()
            .all(|column| !matches!(column.kind, avalio_lite::gradebook::model::ColumnKind::Test { bonus: true, .. })),
        "tests from before bonus tests existed are normal tests"
    );
}

#[test]
fn a_version_2_database_upgrades_with_all_its_data() {
    let (_dir, path) = scratch_copy("db-v2.db");
    let old = Connection::open(&path).unwrap();
    assert_eq!(migrations::schema_version(&old).unwrap(), 2, "the sample really is version 2");
    drop(old);
    let connection = db::open(&path, None).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
    assert_current_sheet(&connection);
}

#[test]
fn a_version_3_database_upgrades_with_all_its_data() {
    let (_dir, path) = scratch_copy("db-v3.db");
    let old = Connection::open(&path).unwrap();
    assert_eq!(migrations::schema_version(&old).unwrap(), 3, "the sample really is version 3");
    drop(old);
    let connection = db::open(&path, None).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
    assert_current_sheet(&connection);
}

#[test]
fn a_version_5_database_upgrades_and_keeps_its_pupils() {
    let (_dir, path) = scratch_copy("db-v5.db");
    let old = Connection::open(&path).unwrap();
    assert_eq!(migrations::schema_version(&old).unwrap(), 5, "the sample really is version 5");
    drop(old);
    let connection = db::open(&path, None).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
    assert_current_sheet(&connection);
    let class = classes::list_classes(&connection).unwrap()[0].id;
    let names: Vec<(String, String)> = pupils::list_class_pupils(&connection, class)
        .unwrap()
        .into_iter()
        .map(|pupil| (pupil.first_name, pupil.surname))
        .collect();
    assert_eq!(
        names,
        [("Ana".to_owned(), String::new()), ("Bea".to_owned(), String::new())],
        "whole names, no surnames"
    );
    assert_eq!(column_order(&connection), "FSmmmm");
    assert_eq!(sheet_widths(&connection), [Some(260), None, None, None, None, Some(300)]);
}

#[test]
fn a_version_6_database_upgrades_with_its_names_first() {
    let (_dir, path) = scratch_copy("db-v6.db");
    let old = Connection::open(&path).unwrap();
    assert_eq!(migrations::schema_version(&old).unwrap(), 6, "the sample really is version 6");
    drop(old);
    let connection = db::open(&path, None).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
    assert_current_sheet(&connection);
    assert_eq!(column_order(&connection), "FSmmmm", "the names come first, where the sheet showed them");
    assert_eq!(sheet_widths(&connection), [Some(260), Some(180), None, None, None, Some(300)], "with their widths");
    let class = classes::list_classes(&connection).unwrap()[0].id;
    let loaded = sheet::load_sheet(&connection, class).unwrap();
    assert_eq!(loaded.rows[0].values[..2], ["Ana", "Pérez"], "the name columns show the pupil's names");
}

#[test]
fn a_version_4_database_upgrades_with_all_its_data() {
    let (_dir, path) = scratch_copy("db-v4.db");
    let old = Connection::open(&path).unwrap();
    assert_eq!(migrations::schema_version(&old).unwrap(), 4, "the sample really is version 4");
    drop(old);
    let connection = db::open(&path, None).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
    assert_current_sheet(&connection);
    assert_eq!(sheet_widths(&connection), vec![None; 6], "every column keeps its default width");
}

// ---------------------------------------------------------------------------------------------
// Excel export

/// Every tab of a workbook as tab-separated text: a `# name` line per tab, then its rows, each
/// cell marked with its type (`n:` number, `s:` text), so a number exported as text shows up.
fn workbook_dump(path: &Path) -> String {
    use calamine::{Data, Reader, Xlsx};
    let mut workbook: Xlsx<_> = calamine::open_workbook(path).unwrap();
    let mut dump = String::new();
    for name in workbook.sheet_names() {
        dump.push_str(&format!("# {name}\n"));
        for row in workbook.worksheet_range(&name).unwrap().rows() {
            let cells: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    Data::Empty => String::new(),
                    Data::Float(number) => format!("n:{number}"),
                    Data::String(text) => format!("s:{text}"),
                    other => panic!("an export only has numbers and text, not {other:?}"),
                })
                .collect();
            dump.push_str(&cells.join("\t"));
            dump.push('\n');
        }
    }
    dump
}

/// The export of the golden class. The .xlsx bytes hold a timestamp, so its contents are compared.
#[test]
fn the_sheet_export_matches_the_golden_dump() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    fill_current(&connection);
    let class = classes::list_classes(&connection).unwrap()[0].id;
    let path = dir.path().join("1A.xlsx");
    avalio_lite::spreadsheet::export::export_sheet(&connection, class, Language::English, &path).unwrap();
    let dump = workbook_dump(&path);
    if updating() {
        fs::write(golden("sheet-export.tsv"), &dump).unwrap();
    }
    let expected = fs::read_to_string(golden("sheet-export.tsv")).unwrap().replace('\r', "");
    assert_eq!(dump, expected, "the export changed; see the module docs");
}

/// The schema a fresh database gets, as SQLite stores it. Changing it needs a new migration, and
/// a deliberate update of this sample.
#[test]
fn the_schema_matches_the_golden_dump() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    let mut statement = connection
        .prepare("SELECT sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap();
    let dump: Vec<String> = statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect();
    let dump = format!("{};\n", dump.join(";\n\n"));
    if updating() {
        fs::write(golden("schema.sql"), &dump).unwrap();
    }
    assert_eq!(dump, fs::read_to_string(golden("schema.sql")).unwrap(), "the schema changed; see the module docs");
}

// ---------------------------------------------------------------------------------------------
// Release signatures

/// A `.sig` line made with a fixed test key: Ed25519 signatures are deterministic, so the line only
/// changes with the format. Signatures already shipped must keep verifying in every later version.
#[test]
fn the_release_signature_format_is_stable() {
    use avalio_lite::update::signature;
    let key = ed25519_dalek::SigningKey::from_bytes(&[7; 32]);
    let (name, hash) = ("avalio-lite-9.9.9-setup.exe", blake3::hash(b"avalio-lite").to_hex().to_string());
    let line = format!("{}\n", signature::sig_line(&signature::sign(&key, name, &hash), name));
    if updating() {
        fs::write(golden("release-v1.sig"), &line).unwrap();
    }
    let expected = fs::read_to_string(golden("release-v1.sig")).unwrap();
    assert_eq!(line, expected, "the signature format changed; see the module docs");
    assert_eq!(signature::verify(&key.verifying_key(), name, &hash, &expected), Ok(()));
}
