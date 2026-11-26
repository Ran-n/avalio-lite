// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:18.501182
//+ Revised: 	2026/11/25 11:43:10.815175
// -----------------------------------------------------------------------------

//! Pupils' pictures in backups: exported and restored with everything else, encrypted with the PIN,
//! and brought along when a backup's classes are added.

use std::fs;

use crate::common::Fixture;
use crate::support::PIN;
use avalio_lite::backup;
use avalio_lite::gradebook::{classes, pictures, pupils};
use avalio_lite::platform::db;
use rusqlite::Connection;

/// Bytes no database page holds by chance.
const ANAS: &[u8] = b"ANAS-PICTURE-BYTES";

fn database(fixture: &Fixture, name: &str) -> Connection {
    db::open(&fixture.path(name), None).unwrap()
}

/// Class `class` with Ana, who has `picture` when given; returns Ana.
fn with_ana(connection: &Connection, class: &str, picture: Option<&[u8]>) -> i64 {
    let class = classes::create_class(connection, class).unwrap();
    let ana = pupils::add_new_pupil(connection, class, "Ana", "Pérez").unwrap();
    if let Some(picture) = picture {
        pictures::set_picture(connection, ana, picture).unwrap();
    }
    ana
}

fn all_pictures(connection: &Connection) -> Vec<Vec<u8>> {
    let mut statement = connection.prepare("SELECT image FROM pupil_pictures ORDER BY pupil_id").unwrap();
    statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
}

#[test]
fn a_backup_holds_the_pictures_encrypted_with_the_pin() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    with_ana(state.connection().unwrap(), "1A", Some(ANAS));
    let holds = |path: &std::path::Path| fs::read(path).unwrap().windows(ANAS.len()).any(|bytes| bytes == ANAS);
    let plain = fixture.path("plain.avalio");
    state.export_backup(&plain).unwrap();
    assert!(holds(&plain));
    state.set_pin(PIN, PIN).unwrap();
    let encrypted = fixture.path("encrypted.avalio");
    state.export_backup(&encrypted).unwrap();
    assert!(!holds(&encrypted) && !holds(&fixture.database()), "neither the backup nor the file shows it");
    let opened = db::open_existing(&encrypted, Some(PIN)).unwrap();
    assert_eq!(all_pictures(&opened), [ANAS]);
}

#[test]
fn added_classes_bring_their_pupils_pictures() {
    let fixture = Fixture::new();
    let (source, target) = (database(&fixture, "source.db"), database(&fixture, "target.db"));
    with_ana(&source, "1A", Some(ANAS));
    backup::add_classes(&target, &source).unwrap();
    assert_eq!(all_pictures(&target), [ANAS]);
}

#[test]
fn a_pupil_already_here_keeps_their_own_picture() {
    let fixture = Fixture::new();
    let (source, target) = (database(&fixture, "source.db"), database(&fixture, "target.db"));
    with_ana(&source, "1A", Some(ANAS));
    with_ana(&target, "2B", Some(b"the one here"));
    backup::add_classes(&target, &source).unwrap();
    assert_eq!(all_pictures(&target), [b"the one here"]);
}

#[test]
fn a_pupil_already_here_without_a_picture_gets_it() {
    let fixture = Fixture::new();
    let (source, target) = (database(&fixture, "source.db"), database(&fixture, "target.db"));
    with_ana(&source, "1A", Some(ANAS));
    let ana = with_ana(&target, "2B", None);
    backup::add_classes(&target, &source).unwrap();
    assert_eq!(pictures::picture(&target, ana).unwrap().as_deref(), Some(ANAS));
}

#[test]
fn a_backup_without_pictures_removes_none() {
    let fixture = Fixture::new();
    let (source, target) = (database(&fixture, "source.db"), database(&fixture, "target.db"));
    with_ana(&source, "1A", None);
    with_ana(&target, "2B", Some(ANAS));
    backup::add_classes(&target, &source).unwrap();
    assert_eq!(all_pictures(&target), [ANAS]);
}
