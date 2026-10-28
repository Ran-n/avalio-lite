// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:06.706242
//+ Revised: 	2026/10/28 16:34:05.740388
// -----------------------------------------------------------------------------

//! Files that outlive an app version, pinned as checked-in samples in `tests/golden/`: the settings
//! file and the database, in every format a user may still have. Each is built from fixed data,
//! so the samples only change when the format does.
//!
//! `AVALIO_UPDATE_GOLDEN=1 cargo test --test golden` rewrites the samples of the *current* formats
//! after a deliberate format change (review the diff, then commit). Samples of older formats are
//! frozen: they stand for files already on users' disks, so they are only ever added, never rebuilt.

use std::fs;
use std::path::{Path, PathBuf};

use avalio_lite::config::{AppConfig, ConfigStore, Preferences, ThemeChoice};
use avalio_lite::db::{self, migrations};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{cells, classes, columns, pupils, sheet};
use avalio_lite::i18n::Language;
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
        },
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
fn a_settings_file_from_before_preferences_loads_with_the_defaults() {
    let dir = tempfile::tempdir().unwrap();
    fs::copy(golden("config-v1.toml"), dir.path().join("config.toml")).unwrap();
    let config = ConfigStore::in_dir(dir.path()).load().unwrap().unwrap();
    assert_eq!(config.database_path, Path::new(r"D:\Marks\avalio-lite.db"));
    assert_eq!(config.preferences, Preferences::default());
}

// ---------------------------------------------------------------------------------------------
// Database

/// Class "1A": Ana (7.5) and Bea (4) in "Exam" (100 %, out of 10), a final grade, and a note.
fn fill_current(connection: &Connection) {
    let class = classes::create_class(connection, "1A").unwrap();
    let ana = pupils::add_new_pupil(connection, class, "Ana").unwrap();
    let bea = pupils::add_new_pupil(connection, class, "Bea").unwrap();
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
}

/// What the current golden database shows for class "1A".
fn assert_current_sheet(connection: &Connection) {
    let class = classes::list_classes(connection).unwrap()[0].id;
    let loaded = sheet::load_sheet(connection, class).unwrap();
    let titles: Vec<&str> = loaded.columns.iter().map(|column| column.title.as_str()).collect();
    assert_eq!(titles, ["Exam", "Project", "Final grade", "Notes"]);
    let names: Vec<&str> = loaded.rows.iter().map(|row| row.pupil.name.as_str()).collect();
    assert_eq!(names, ["Ana", "Bea"]);
    assert_eq!(loaded.rows[0].values, ["7,5", "10", "", ""]);
    assert_eq!(loaded.rows[1].values, ["4", "", "", "Needs glasses"]);
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
    let titles: Vec<&str> = loaded.columns.iter().map(|column| column.title.as_str()).collect();
    assert_eq!(titles, ["Exam", "Final grade"]);
    assert_eq!(loaded.rows[0].pupil.name, "Ana");
    assert_eq!(loaded.rows[0].values, ["7,5", ""]);
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
