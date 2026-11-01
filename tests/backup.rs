// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 09:42:12.935204
//+ Revised: 	2026/10/29 14:45:37.325875
// -----------------------------------------------------------------------------

//! `.avalio` backups: exporting everything to one file, read back from the real file on disk.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use rusqlite::Connection;

mod common;
use common::Fixture;

use avalio_lite::app::dialogs::FileKind;
use avalio_lite::app::state::AppState;
use avalio_lite::backup;
use avalio_lite::db::{self, migrations};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::compute::evaluate_row;
use avalio_lite::gradebook::model::{ColumnKind, ColumnType, Sheet};
use avalio_lite::gradebook::{cells, classes, columns, pupils, sheet};
use avalio_lite::i18n::Language;

const PIN: &str = "246810";

/// A database with the sample class and a second, empty one.
fn filled(fixture: &Fixture) -> AppState {
    let state = fixture.set_up();
    let connection = state.connection().unwrap();
    common::sample_class(connection);
    classes::create_class(connection, "1º ESO B").unwrap();
    state
}

/// Every class with its whole sheet (columns, pupils in order, typed values), by name.
fn everything(connection: &Connection) -> Vec<(String, Sheet)> {
    let mut all = Vec::new();
    for class in classes::list_classes(connection).unwrap() {
        all.push((class.name, sheet::load_sheet(connection, class.id).unwrap()));
    }
    all
}

fn partial(destination: &Path) -> PathBuf {
    PathBuf::from(format!("{}.partial", destination.display()))
}

#[test]
fn a_backup_holds_everything() {
    let fixture = Fixture::new();
    let state = filled(&fixture);
    let destination = fixture.path("backups").join("all.avalio");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    state.export_backup(&destination).unwrap();

    let copy = db::open_existing(&destination, None).unwrap();
    let original = everything(state.connection().unwrap());
    assert_eq!(original.len(), 2);
    assert_eq!(everything(&copy), original);
    assert!(!partial(&destination).exists());
}

#[test]
fn a_backup_of_an_encrypted_database_needs_its_pin() {
    let fixture = Fixture::new();
    let mut state = filled(&fixture);
    state.set_pin(PIN, PIN).unwrap();
    let destination = fixture.path("secret.avalio");
    state.export_backup(&destination).unwrap();

    assert!(db::is_encrypted(&destination).unwrap());
    assert!(matches!(db::open_existing(&destination, None), Err(AppError::WrongPin)));
    assert!(matches!(db::open_existing(&destination, Some("135790")), Err(AppError::WrongPin)));
    let copy = db::open_existing(&destination, Some(PIN)).unwrap();
    assert_eq!(everything(&copy), everything(state.connection().unwrap()));
}

#[test]
fn a_backup_replaces_an_existing_file_only_once_complete() {
    let fixture = Fixture::new();
    let state = filled(&fixture);

    // a folder of the chosen name: the final rename fails, and the folder is left as it was
    let taken = fixture.path("taken.avalio");
    fs::create_dir_all(&taken).unwrap();
    fs::write(taken.join("keep.txt"), "mine").unwrap();
    let result = state.export_backup(&taken);
    assert!(matches!(result, Err(AppError::Io { .. })), "{result:?}");
    assert_eq!(fs::read_to_string(taken.join("keep.txt")).unwrap(), "mine");
    assert!(!partial(&taken).exists(), "the half-done copy is removed");

    // an older backup of the chosen name: replaced by the complete new one
    let destination = fixture.path("weekly.avalio");
    let other = Fixture::new();
    other.set_up().export_backup(&destination).unwrap();
    assert!(everything(&db::open_existing(&destination, None).unwrap()).is_empty());
    state.export_backup(&destination).unwrap();
    assert_eq!(everything(&db::open_existing(&destination, None).unwrap()).len(), 2);
}

#[test]
fn a_leftover_partial_file_is_replaced() {
    let fixture = Fixture::new();
    let state = filled(&fixture);
    let destination = fixture.path("all.avalio");
    fs::write(partial(&destination), b"from a crash").unwrap();
    state.export_backup(&destination).unwrap();
    assert!(!partial(&destination).exists());
    assert_eq!(everything(&db::open_existing(&destination, None).unwrap()).len(), 2);
}

#[test]
fn a_leftover_partial_file_that_cannot_be_removed_stops_the_backup() {
    let fixture = Fixture::new();
    let state = filled(&fixture);
    let destination = fixture.path("all.avalio");
    fs::write(partial(&destination), b"from a crash").unwrap();
    let _held = common::hold_exclusively(&partial(&destination));
    assert!(matches!(state.export_backup(&destination), Err(AppError::Io { .. })));
    assert!(!destination.exists());
}

#[test]
fn a_backup_needs_an_open_database() {
    let fixture = Fixture::new();
    let state = fixture.state();
    assert!(matches!(state.export_backup(&fixture.path("x.avalio")), Err(AppError::NoDatabaseOpen)));
}

#[test]
fn file_kinds_filter_their_extensions_and_name_them() {
    // a backup can also be a plain database file, such as a copied avalio-lite.db: the same format
    assert_eq!(FileKind::Backup.open_extensions(), [backup::EXTENSION, "db"]);
    assert_eq!(FileKind::Backup.save_extensions(), [backup::EXTENSION]);
    assert_eq!(FileKind::Database.open_extensions(), ["db"]);
    assert_eq!(FileKind::Database.save_extensions(), ["db"]);
    assert_eq!(FileKind::Backup.filter_name(Language::English), "avalio-lite backup");
    assert_eq!(FileKind::Backup.filter_name(Language::Galician), "Copia de seguranza de avalio-lite");
    assert_eq!(FileKind::Database.filter_name(Language::English), "avalio-lite database");
    assert_eq!(FileKind::Database.filter_name(Language::Galician), "Base de datos de avalio-lite");
}

#[test]
fn the_default_name_carries_the_date() {
    let date = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
    assert_eq!(backup::default_file_name(date), "avalio-lite-backup-2026-10-28.avalio");
    assert!(backup::default_file_name(date).ends_with(&format!(".{}", backup::EXTENSION)));
}

#[test]
fn a_backup_keeps_the_schema_version() {
    let fixture = Fixture::new();
    let state = filled(&fixture);
    let destination = fixture.path("all.avalio");
    state.export_backup(&destination).unwrap();
    assert_eq!(db::stored_version(&destination, None).unwrap(), migrations::latest_version());
}

// ---------------------------------------------------------------------------------------------
// Restoring

/// The `.restore` copy the app opens a backup through, next to the database.
fn restore_copy(fixture: &Fixture) -> PathBuf {
    PathBuf::from(format!("{}.restore", fixture.database().display()))
}

/// A backup of a filled database of its own, encrypted with `pin` when given.
fn backup_of_another(pin: Option<&str>) -> (Fixture, PathBuf) {
    let other = Fixture::new();
    let mut state = filled(&other);
    if let Some(pin) = pin {
        state.set_pin(pin, pin).unwrap();
    }
    let file = other.path("theirs.avalio");
    state.export_backup(&file).unwrap();
    state.shutdown();
    (other, file)
}

fn class_names(state: &AppState) -> Vec<String> {
    classes::list_classes(state.connection().unwrap()).unwrap().into_iter().map(|class| class.name).collect()
}

#[test]
fn restoring_replaces_everything() {
    let (_other, file) = backup_of_another(None);
    let expected = everything(&db::open_existing(&file, None).unwrap());
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    classes::create_class(state.connection().unwrap(), "Only here").unwrap();

    let opened = state.open_backup(&file, None).unwrap();
    assert_eq!(opened.contents(), backup::BackupContents { classes: 2, pupils: 5 });
    assert_eq!(opened.file_name(), "theirs.avalio");
    state.restore_backup(opened).unwrap();

    assert_eq!(everything(state.connection().unwrap()), expected);
    assert!(!restore_copy(&fixture).exists());
    state.shutdown();
    let mut restarted = fixture.state();
    restarted.start();
    assert_eq!(class_names(&restarted), ["1º ESO B", "2º ESO A"], "the restore is on disk");
}

#[test]
fn restoring_keeps_the_current_pin() {
    // an encrypted database and a plain backup: still encrypted with its own PIN afterwards
    let (_other, plain) = backup_of_another(None);
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin("1111", "1111").unwrap();
    let opened = state.open_backup(&plain, None).unwrap();
    state.restore_backup(opened).unwrap();
    assert!(state.is_protected());
    state.shutdown();
    assert!(matches!(db::open_existing(&fixture.database(), None), Err(AppError::WrongPin)));
    assert_eq!(everything(&db::open_existing(&fixture.database(), Some("1111")).unwrap()).len(), 2);

    // a plain database and a backup made with another PIN: plain afterwards
    let (_other, secret) = backup_of_another(Some("2222"));
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    assert!(matches!(state.open_backup(&secret, None), Err(AppError::WrongPin)));
    let opened = state.open_backup(&secret, Some("2222")).unwrap();
    state.restore_backup(opened).unwrap();
    assert!(!state.is_protected());
    state.shutdown();
    assert!(!db::is_encrypted(&fixture.database()).unwrap());
    assert_eq!(everything(&db::open_existing(&fixture.database(), None).unwrap()).len(), 2);
}

#[test]
fn a_backup_of_this_database_opens_with_its_pin_without_asking() {
    let fixture = Fixture::new();
    let mut state = filled(&fixture);
    state.set_pin(PIN, PIN).unwrap();
    let file = fixture.path("mine.avalio");
    state.export_backup(&file).unwrap();
    let opened = state.open_backup(&file, None).unwrap();
    assert_eq!(opened.contents().classes, 2);
}

#[test]
fn a_wrong_backup_pin_is_refused() {
    let (_other, secret) = backup_of_another(Some("2222"));
    let fixture = Fixture::new();
    let state = fixture.set_up();
    assert!(matches!(state.open_backup(&secret, Some("3333")), Err(AppError::WrongPin)));
    assert!(!restore_copy(&fixture).exists());
}

#[test]
fn an_older_backup_is_upgraded_and_the_file_is_untouched() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/db-v1.db");
    let before = fs::read(&golden).unwrap();
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let opened = state.open_backup(&golden, None).unwrap();
    state.restore_backup(opened).unwrap();
    assert_eq!(fs::read(&golden).unwrap(), before, "the chosen file is never changed");

    let connection = state.connection().unwrap();
    assert_eq!(migrations::schema_version(connection).unwrap(), migrations::latest_version());
    let class = classes::list_classes(connection).unwrap()[0].id;
    let loaded = sheet::load_sheet(connection, class).unwrap();
    let titles: Vec<&str> = loaded.columns.iter().map(|column| column.title.as_str()).collect();
    assert_eq!(titles, ["Exam", "Final grade"]);
    assert_eq!(loaded.rows[0].pupil.name, "Ana");
    assert_eq!(loaded.rows[0].values, ["7,5", ""]);
}

#[test]
fn a_newer_backup_is_refused() {
    let (other, file) = backup_of_another(None);
    let newer = other.path("newer.avalio");
    fs::copy(&file, &newer).unwrap();
    let future = migrations::latest_version() + 1;
    Connection::open(&newer).unwrap().pragma_update(None, "user_version", future as i64).unwrap();

    let fixture = Fixture::new();
    let state = fixture.set_up();
    let result = state.open_backup(&newer, None);
    assert!(matches!(result, Err(AppError::NewerSchema { found, .. }) if found == future), "{:?}", result.err());
    assert!(!restore_copy(&fixture).exists());
}

#[test]
fn a_file_that_is_not_a_database_is_refused() {
    let fixture = Fixture::new();
    let state = fixture.set_up();
    for (name, contents) in [
        ("empty.avalio", Vec::new()),
        ("short.avalio", b"hello".to_vec()),
        ("notes.avalio", "Notes for June: nothing here is a database. ".repeat(40).into_bytes()),
    ] {
        let file = fixture.path(name);
        fs::write(&file, contents).unwrap();
        let result = state.open_backup(&file, None);
        assert!(matches!(&result, Err(AppError::NotADatabase(path)) if *path == file), "{name}: {:?}", result.err());
        assert!(!restore_copy(&fixture).exists(), "{name}");
    }
}

#[test]
fn a_damaged_backup_is_refused() {
    let (other, file) = backup_of_another(None);
    let damaged = other.path("damaged.avalio");
    let mut bytes = fs::read(&file).unwrap();
    // scribble over the second page, past the header SQLite needs to open the file at all
    for byte in &mut bytes[4096..4096 + 200] {
        *byte = 0x5A;
    }
    fs::write(&damaged, bytes).unwrap();

    let fixture = Fixture::new();
    let state = fixture.set_up();
    let result = state.open_backup(&damaged, None);
    assert!(result.is_err(), "a damaged backup can't be opened");
    assert!(!matches!(result, Err(AppError::WrongPin)), "{:?}", result.err());
    assert!(!restore_copy(&fixture).exists());
}

#[test]
fn a_missing_backup_file_is_reported() {
    let fixture = Fixture::new();
    let state = fixture.set_up();
    let gone = fixture.path("gone.avalio");
    assert!(matches!(state.open_backup(&gone, None), Err(AppError::Io { path, .. }) if path == gone));
}

#[test]
fn opening_a_backup_needs_a_chosen_database() {
    let (_other, file) = backup_of_another(None);
    let fixture = Fixture::new();
    assert!(matches!(fixture.state().open_backup(&file, None), Err(AppError::NoDatabaseOpen)));
}

#[test]
fn a_failed_swap_keeps_the_current_database() {
    let (_other, file) = backup_of_another(None);
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    classes::create_class(state.connection().unwrap(), "Only here").unwrap();
    let opened = state.open_backup(&file, None).unwrap();
    // another program has the database open, so it can't be renamed aside
    let held = common::hold_against_removal(&fixture.database());
    let result = state.restore_backup(opened);
    assert!(matches!(result, Err(AppError::Io { .. })), "{result:?}");
    assert_eq!(class_names(&state), ["Only here"], "the current database is open again, unchanged");
    assert!(!restore_copy(&fixture).exists());
    drop(held);
    state.shutdown();
    let mut restarted = fixture.state();
    restarted.start();
    assert_eq!(class_names(&restarted), ["Only here"]);
}

#[test]
fn the_restore_copy_never_stays_behind() {
    let (_other, secret) = backup_of_another(Some("2222"));
    let fixture = Fixture::new();
    let state = fixture.set_up();
    // asked for a PIN, then given a wrong one, then cancelled (the opened backup dropped)
    assert!(state.open_backup(&secret, None).is_err());
    assert!(!restore_copy(&fixture).exists());
    assert!(state.open_backup(&secret, Some("0000")).is_err());
    assert!(!restore_copy(&fixture).exists());
    let opened = state.open_backup(&secret, Some("2222")).unwrap();
    assert!(restore_copy(&fixture).exists(), "the copy is open while the question is asked");
    drop(opened);
    assert!(!restore_copy(&fixture).exists());
    // a copy left by a crash is replaced
    fs::write(restore_copy(&fixture), b"from a crash").unwrap();
    drop(state.open_backup(&secret, Some("2222")).unwrap());
    assert!(!restore_copy(&fixture).exists());
}

// ---------------------------------------------------------------------------------------------
// Adding the classes of a backup

/// A fresh database file of its own in `fixture`.
fn database(fixture: &Fixture, name: &str) -> Connection {
    db::open(&fixture.path(name), None).unwrap()
}

fn class_id(connection: &Connection, name: &str) -> i64 {
    classes::list_classes(connection).unwrap().into_iter().find(|class| class.name == name).unwrap().id
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0)).unwrap()
}

/// Everything a class's sheet shows, without row ids: column titles and settings (a rounded
/// column's source as a position), pupils in order with their typed values, and every calculated
/// cell. Two classes with the same shape look the same to the user.
fn shape(connection: &Connection, class: i64) -> String {
    let sheet = sheet::load_sheet(connection, class).unwrap();
    let position = |id: i64| sheet.columns.iter().position(|column| column.id == id);
    let columns: Vec<String> = sheet
        .columns
        .iter()
        .map(|column| match column.kind {
            ColumnKind::Round { source, decimals } => {
                format!("{} round {:?} {decimals}", column.title, source.and_then(position))
            }
            kind => format!("{} {kind:?}", column.title),
        })
        .collect();
    let rows: Vec<String> = sheet
        .rows
        .iter()
        .map(|row| format!("{} {:?} {:?}", row.pupil.name, row.values, evaluate_row(&sheet.columns, &row.values)))
        .collect();
    format!("{columns:#?}\n{rows:#?}")
}

#[test]
fn added_classes_carry_columns_pupils_and_marks() {
    let fixture = Fixture::new();
    let source = database(&fixture, "source.db");
    common::sample_class(&source);
    classes::create_class(&source, "1º ESO B").unwrap();
    let target = database(&fixture, "target.db");
    classes::create_class(&target, "Other").unwrap();

    let summary = backup::add_classes(&target, &source).unwrap();
    assert_eq!(summary.class_ids.len(), 2);
    assert_eq!((summary.pupils, summary.existing_pupils), (5, 0));
    assert!(summary.renamed.is_empty());
    for name in ["2º ESO A", "1º ESO B"] {
        assert_eq!(shape(&target, class_id(&target, name)), shape(&source, class_id(&source, name)), "{name}");
    }
    assert_eq!(classes::list_classes(&target).unwrap().len(), 3, "the class already here stays");
}

#[test]
fn taken_names_get_the_lowest_free_number() {
    let fixture = Fixture::new();
    let target = database(&fixture, "target.db");
    classes::create_class(&target, "1A").unwrap();
    classes::create_class(&target, "1A (2)").unwrap();
    classes::create_class(&target, "1A (4)").unwrap();
    assert_eq!(classes::free_class_name(&target, "1B").unwrap(), "1B");
    assert_eq!(classes::free_class_name(&target, "  1A ").unwrap(), "1A (3)");
    assert!(matches!(classes::free_class_name(&target, " "), Err(AppError::EmptyName)));

    let source = database(&fixture, "source.db");
    classes::create_class(&source, "1A").unwrap();
    let summary = backup::add_classes(&target, &source).unwrap();
    assert_eq!(summary.renamed, [("1A".to_owned(), "1A (3)".to_owned())]);
    assert_eq!(classes::class_name(&target, summary.class_ids[0]).unwrap(), "1A (3)");
}

#[test]
fn pupils_match_by_unique_name_and_shared_pupils_stay_shared() {
    let fixture = Fixture::new();
    let target = database(&fixture, "target.db");
    let here = classes::create_class(&target, "Here").unwrap();
    let ana = pupils::add_new_pupil(&target, here, "Ana").unwrap();
    pupils::add_new_pupil(&target, here, "Brais").unwrap();
    pupils::add_new_pupil(&target, here, "Brais").unwrap();

    let source = database(&fixture, "source.db");
    let x = classes::create_class(&source, "X").unwrap();
    let y = classes::create_class(&source, "Y").unwrap();
    let shared = pupils::add_new_pupil(&source, x, " ANA ").unwrap();
    pupils::add_existing_pupil(&source, y, shared).unwrap();
    pupils::add_new_pupil(&source, x, "Brais").unwrap();
    // two different pupils of the same name, neither of them here: two new pupils
    pupils::add_new_pupil(&source, x, "Dani").unwrap();
    pupils::add_new_pupil(&source, y, "Dani").unwrap();

    let summary = backup::add_classes(&target, &source).unwrap();
    assert_eq!((summary.pupils, summary.existing_pupils), (4, 1));
    let names = |class: &str| -> Vec<(i64, String)> {
        let pupils = pupils::list_class_pupils(&target, class_id(&target, class)).unwrap();
        pupils.into_iter().map(|pupil| (pupil.id, pupil.name)).collect()
    };
    let (in_x, in_y) = (names("X"), names("Y"));
    assert_eq!(in_x[0], (ana, "Ana".to_owned()), "the one Ana here joins, under her name here");
    assert_eq!(in_y[0].0, ana, "a pupil shared in the backup stays one pupil");
    assert_eq!(in_x.iter().map(|(_, name)| name.as_str()).collect::<Vec<_>>(), ["Ana", "Brais", "Dani"]);
    assert_eq!(pupils::pupils_named(&target, "brais").unwrap().len(), 3, "two Brais here: a new one");
    assert_ne!(in_x[2].0, in_y[1].0, "the two Danis stay two pupils");
    assert_eq!(count(&target, "pupils"), 6);
}

#[test]
fn rounded_columns_keep_their_source_even_to_the_right() {
    let fixture = Fixture::new();
    let source = database(&fixture, "source.db");
    let class = classes::create_class(&source, "1A").unwrap();
    let round = columns::add_column(&source, class, ColumnType::Round, Language::English).unwrap();
    let exam = columns::add_column(&source, class, ColumnType::Test, Language::English).unwrap();
    columns::set_round_source(&source, round, Some(exam)).unwrap();
    columns::set_decimals(&source, round, "1").unwrap();

    let target = database(&fixture, "target.db");
    let summary = backup::add_classes(&target, &source).unwrap();
    let added = columns::list_columns(&target, summary.class_ids[0]).unwrap();
    assert_eq!(added[0].kind, ColumnKind::Round { source: Some(added[1].id), decimals: 1 });
}

#[test]
fn a_failure_adds_nothing() {
    let fixture = Fixture::new();
    let source = database(&fixture, "source.db");
    common::sample_class(&source);
    let target = database(&fixture, "target.db");
    let here = classes::create_class(&target, "Here").unwrap();
    pupils::add_new_pupil(&target, here, "Ana Pérez").unwrap();
    let before = ["classes", "pupils", "class_pupils", "sheet_columns", "cells"].map(|table| count(&target, table));
    // the add fails half way through the marks
    target
        .execute_batch(
            "CREATE TEMP TRIGGER fail_half_way BEFORE INSERT ON cells WHEN (SELECT COUNT(*) FROM cells) >= 4
             BEGIN SELECT RAISE(ABORT, 'test'); END",
        )
        .unwrap();
    assert!(matches!(backup::add_classes(&target, &source), Err(AppError::Database(_))));
    let after = ["classes", "pupils", "class_pupils", "sheet_columns", "cells"].map(|table| count(&target, table));
    assert_eq!(after, before);
}

#[test]
fn an_empty_backup_adds_nothing_and_says_so() {
    let fixture = Fixture::new();
    let source = database(&fixture, "source.db");
    let target = database(&fixture, "target.db");
    classes::create_class(&target, "Here").unwrap();
    assert_eq!(backup::add_classes(&target, &source).unwrap(), backup::AddSummary::default());
    assert_eq!(count(&target, "classes"), 1);
}

#[test]
fn an_older_backup_adds_its_classes() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/db-v1.db");
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let opened = state.open_backup(&golden, None).unwrap();
    let summary = state.add_backup_classes(opened).unwrap();
    assert!(!restore_copy(&fixture).exists(), "the backup's copy is deleted afterwards");
    let connection = state.connection().unwrap();
    let loaded = sheet::load_sheet(connection, summary.class_ids[0]).unwrap();
    let titles: Vec<&str> = loaded.columns.iter().map(|column| column.title.as_str()).collect();
    assert_eq!(titles, ["Exam", "Final grade"]);
    assert_eq!(loaded.rows[0].pupil.name, "Ana");
    assert_eq!(loaded.rows[0].values, ["7,5", ""]);
    state.set_pin(PIN, PIN).unwrap();
    assert!(state.is_protected(), "adding classes leaves the database usable as before");
}

#[test]
fn adding_classes_needs_an_open_database() {
    let (_other, file) = backup_of_another(None);
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let opened = state.open_backup(&file, None).unwrap();
    state.set_pin(PIN, PIN).unwrap();
    assert!(state.lock());
    assert!(matches!(state.add_backup_classes(opened), Err(AppError::NoDatabaseOpen)));
    assert!(!restore_copy(&fixture).exists());
}

const POOL: [&str; 4] = ["Ana", "Brais", "Carla", "Dani"];

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(16))]

    /// Adding a database's own backup gives every class twice (the copy named " (2)") with the same
    /// sheet, and no new pupils, since every name already belongs to exactly one pupil.
    #[test]
    fn adding_a_backup_of_itself_doubles_the_classes(
        membership in proptest::collection::vec(proptest::collection::vec(proptest::bool::ANY, 4), 1..4),
        marks in proptest::collection::vec(proptest::option::of(0u8..11), 16),
    ) {
        let fixture = Fixture::new();
        let target = database(&fixture, "target.db");
        let pool: Vec<i64> = POOL
            .iter()
            .map(|name| {
                target.execute("INSERT INTO pupils (name) VALUES (?1)", [name]).unwrap();
                target.last_insert_rowid()
            })
            .collect();
        for (index, members) in membership.iter().enumerate() {
            let class = classes::create_class(&target, &format!("Class {index}")).unwrap();
            let exam = columns::add_column(&target, class, ColumnType::Test, Language::English).unwrap();
            columns::add_column(&target, class, ColumnType::Round, Language::English).unwrap();
            for (pupil, _) in pool.iter().zip(members).filter(|(_, member)| **member) {
                pupils::add_existing_pupil(&target, class, *pupil).unwrap();
                if let Some(mark) = marks[index * 4 + pool.iter().position(|id| id == pupil).unwrap()] {
                    cells::set_cell(&target, exam, *pupil, &mark.to_string()).unwrap();
                }
            }
        }
        let file = fixture.path("itself.avalio");
        backup::export(&target, &file, None).unwrap();
        let source = db::open_existing(&file, None).unwrap();

        let summary = backup::add_classes(&target, &source).unwrap();
        proptest::prop_assert_eq!(summary.class_ids.len(), membership.len());
        proptest::prop_assert_eq!(count(&target, "pupils"), 4, "no new pupils");
        for index in 0..membership.len() {
            let name = format!("Class {index}");
            let copy = class_id(&target, &format!("{name} (2)"));
            proptest::prop_assert_eq!(shape(&target, copy), shape(&target, class_id(&target, &name)));
        }
    }
}
