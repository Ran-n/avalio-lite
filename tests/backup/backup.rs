// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.683927
//+ Revised: 	2026/11/23 13:00:36.683927
// -----------------------------------------------------------------------------

//! Making a backup: the content, the encryption, replacing a file and the leftover partial copies.

use crate::common::Fixture;
use crate::support::{PIN, everything, filled};
use avalio_lite::backup;
use avalio_lite::error::AppError;
use avalio_lite::i18n::Language;
use avalio_lite::platform::db::{self, migrations};
use avalio_lite::window::dialogs::FileKind;
use chrono::NaiveDate;
use std::fs;
use std::path::{Path, PathBuf};

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
    let _held = crate::common::hold_exclusively(&partial(&destination));
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
    assert_eq!(FileKind::Backup.filter_name(Language::English), "avalio lite backup");
    assert_eq!(FileKind::Backup.filter_name(Language::Galician), "Copia de seguranza de avalio lite");
    assert_eq!(FileKind::Database.filter_name(Language::English), "avalio lite database");
    assert_eq!(FileKind::Database.filter_name(Language::Galician), "Base de datos de avalio lite");
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

#[test]
fn regression_a_failed_backup_copy_leaves_no_partial_file() {
    // a copy cut short stayed next to the database, possibly readable
    let fixture = Fixture::new();
    let copy = fixture.path("copy.restore");
    let missing = fixture.path("missing.avalio");
    assert!(matches!(backup::open_copy(&missing, &copy, &[None]), Err(AppError::Io { path, .. }) if path == missing));
    assert!(!copy.exists());
}
