// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:32.532831
//+ Revised: 	2026/11/02 14:00:10.248567
// -----------------------------------------------------------------------------

use std::fs;

use avalio_lite::error::AppError;
use avalio_lite::storage::{ensure_parent, move_file, same_location};

mod common;

#[test]
fn move_file_moves_contents_into_new_folder() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("a.log");
    let destination = dir.path().join("new").join("a.log");
    fs::write(&source, b"hello").unwrap();
    move_file(&source, &destination).unwrap();
    assert!(!source.exists());
    assert_eq!(fs::read(&destination).unwrap(), b"hello");
}

#[test]
fn move_file_refuses_to_overwrite_and_keeps_both() {
    let dir = tempfile::tempdir().unwrap();
    let (source, destination) = (dir.path().join("a"), dir.path().join("b"));
    fs::write(&source, b"new").unwrap();
    fs::write(&destination, b"old").unwrap();
    assert!(matches!(move_file(&source, &destination), Err(AppError::AlreadyExists(_))));
    assert_eq!(fs::read(&source).unwrap(), b"new");
    assert_eq!(fs::read(&destination).unwrap(), b"old");
}

#[test]
fn move_file_reports_missing_source() {
    let dir = tempfile::tempdir().unwrap();
    let result = move_file(&dir.path().join("missing"), &dir.path().join("b"));
    assert!(matches!(result, Err(AppError::NotFound(_))));
}

#[test]
fn same_location_sees_through_different_spellings() {
    let dir = tempfile::tempdir().unwrap();
    let spelled_differently = dir.path().join("x").join("..");
    fs::create_dir(dir.path().join("x")).unwrap();
    assert!(same_location(dir.path(), &spelled_differently));
    assert!(!same_location(dir.path(), &dir.path().join("x")));
}

#[test]
fn move_file_reports_a_destination_folder_it_cannot_create() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("a.log");
    fs::write(&source, b"hello").unwrap();
    let in_the_way = dir.path().join("folder");
    fs::write(&in_the_way, b"a file where the folder should be").unwrap();
    let result = move_file(&source, &in_the_way.join("a.log"));
    assert!(matches!(result, Err(AppError::Io { .. })), "{result:?}");
    assert_eq!(fs::read(&source).unwrap(), b"hello", "the original stays");
}

#[test]
fn move_file_reports_an_original_it_cannot_remove() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("a.log");
    let destination = dir.path().join("new").join("a.log");
    fs::write(&source, b"hello").unwrap();
    let _held = common::hold_against_removal(&source);
    let result = move_file(&source, &destination);
    assert!(matches!(result, Err(AppError::Io { ref path, .. }) if *path == source), "{result:?}");
    assert!(!destination.exists(), "the copy goes, so trying again isn't blocked by it");
    assert_eq!(fs::read(&source).unwrap(), b"hello");
}

#[test]
fn regression_a_move_that_failed_can_be_tried_again() {
    // a complete copy stayed when the original couldn't be deleted, and every retry then failed
    // with "already exists"
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("a.log");
    let destination = dir.path().join("new").join("a.log");
    fs::write(&source, b"hello").unwrap();
    let held = common::hold_against_removal(&source);
    assert!(move_file(&source, &destination).is_err());
    drop(held);
    move_file(&source, &destination).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"hello");
    assert!(!source.exists());
}

#[test]
fn ensure_parent_creates_the_folder_and_skips_paths_without_one() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a").join("b").join("x.db");
    ensure_parent(&file).unwrap();
    assert!(dir.path().join("a").join("b").is_dir());
    ensure_parent(std::path::Path::new(r"C:\")).unwrap();
    ensure_parent(std::path::Path::new("")).unwrap();
}

#[test]
fn same_location_compares_spelling_when_the_paths_dont_exist() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    assert!(same_location(&missing, &missing));
    assert!(!same_location(&missing, &dir.path().join("other")));
}

#[cfg(windows)]
#[test]
fn a_move_whose_copy_fails_leaves_the_source_and_no_destination() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = tempfile::tempdir().unwrap();
    let (source, destination) = (dir.path().join("a.log"), dir.path().join("b.log"));
    fs::write(&source, b"hello").unwrap();
    // held open without sharing, so the copy can't read it
    let _held = fs::OpenOptions::new().read(true).write(true).share_mode(0).open(&source).unwrap();
    assert!(matches!(move_file(&source, &destination), Err(AppError::Io { .. })));
    assert!(!destination.exists(), "no partial copy is left");
    assert!(source.is_file());
}
