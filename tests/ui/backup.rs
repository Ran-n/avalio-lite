// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:30.825633
//+ Revised: 	2026/11/23 13:00:30.825633
// -----------------------------------------------------------------------------

//! Settings › backup: saving a backup and where the next one goes.

use crate::driver::{click, english};
use crate::fixtures::seeded;
use crate::screens::open_backup_settings;
use avalio_lite::backup;
use avalio_lite::gradebook::classes;
use avalio_lite::i18n::{DataMessage, Message};
use avalio_lite::platform::db;
use avalio_lite::window::dialogs::FileKind;
use std::fs;

#[test]
fn export_everything_saves_a_backup_and_says_where() {
    let ui = seeded();
    open_backup_settings(&ui);
    let destination = ui.fixture.path("backups").join("mine.avalio");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    ui.dialogs.save_file_next(Some(&destination));
    click(&ui, "Export everything…");

    let copy = db::open_existing(&destination, None).unwrap();
    assert_eq!(classes::list_classes(&copy).unwrap().len(), 1);
    let path = destination.display().to_string();
    assert_eq!(
        ui.session.window.get_status_message(),
        english(&Message::Data(DataMessage::BackupExported { path: &path }))
    );
    let requests = ui.dialogs.save_requests.borrow();
    let (kind, _, name) = &requests[0];
    assert_eq!(*kind, FileKind::Backup);
    assert_eq!(*name, backup::default_file_name(chrono::Local::now().date_naive()));
}

#[test]
fn the_next_backup_opens_where_the_last_one_went() {
    let ui = seeded();
    open_backup_settings(&ui);
    let folder = ui.fixture.path("backups");
    fs::create_dir_all(&folder).unwrap();
    ui.dialogs.save_file_next(Some(&folder.join("first.avalio")));
    click(&ui, "Export everything…");
    ui.dialogs.save_file_next(None);
    click(&ui, "Export everything…");
    assert_eq!(ui.dialogs.save_requests.borrow()[1].1, folder);
}

#[test]
fn a_cancelled_backup_does_nothing() {
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.save_file_next(None);
    click(&ui, "Export everything…");
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(fs::read_dir(ui.fixture.root.path()).unwrap().count(), 3, "only config, data and logs");
}

#[test]
fn a_failed_backup_shows_the_error() {
    let ui = seeded();
    open_backup_settings(&ui);
    // a folder already has the chosen name, so the backup can't be put there
    let taken = ui.fixture.path("taken.avalio");
    fs::create_dir_all(&taken).unwrap();
    ui.dialogs.save_file_next(Some(&taken));
    click(&ui, "Export everything…");
    assert!(ui.session.window.get_error_message().contains("taken.avalio"));
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(taken.is_dir());
}
