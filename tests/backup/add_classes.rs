// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.583684
//+ Revised: 	2026/11/24 17:54:06.822182
// -----------------------------------------------------------------------------

//! Adding the classes of a backup: columns, pupils matched by name, and the names taken.

use crate::common::Fixture;
use crate::support::{PIN, backup_of_another, restore_copy, version_ten_backup_across_classes};
use avalio_lite::backup;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::compute::evaluate_row;
use avalio_lite::gradebook::model::{ColumnKind, ColumnType};
use avalio_lite::gradebook::{cells, classes, columns, pupils, sheet};
use avalio_lite::i18n::Language;
use avalio_lite::platform::db::{self};
use rusqlite::Connection;
use std::path::Path;

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
        .map(|row| {
            format!("{} {:?} {:?}", row.pupil.full_name(), row.values, evaluate_row(&sheet.columns, &row.values))
        })
        .collect();
    format!("{columns:#?}\n{rows:#?}")
}

#[test]
fn added_classes_carry_columns_pupils_and_marks() {
    let fixture = Fixture::new();
    let source = database(&fixture, "source.db");
    crate::common::sample_class(&source);
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
    let ana = pupils::add_new_pupil(&target, here, "Ana", "").unwrap();
    pupils::add_new_pupil(&target, here, "Brais", "").unwrap();
    pupils::add_new_pupil(&target, here, "Brais", "").unwrap();

    let source = database(&fixture, "source.db");
    let x = classes::create_class(&source, "X").unwrap();
    let y = classes::create_class(&source, "Y").unwrap();
    let shared = pupils::add_new_pupil(&source, x, " ANA ", "").unwrap();
    pupils::add_existing_pupil(&source, y, shared).unwrap();
    pupils::add_new_pupil(&source, x, "Brais", "").unwrap();
    // two different pupils of the same name, neither of them here: two new pupils
    pupils::add_new_pupil(&source, x, "Dani", "").unwrap();
    pupils::add_new_pupil(&source, y, "Dani", "").unwrap();

    let summary = backup::add_classes(&target, &source).unwrap();
    assert_eq!((summary.pupils, summary.existing_pupils), (4, 1));
    let names = |class: &str| -> Vec<(i64, String)> {
        let pupils = pupils::list_class_pupils(&target, class_id(&target, class)).unwrap();
        pupils.into_iter().map(|pupil| (pupil.id, pupil.full_name())).collect()
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
    // after the first-name and surname columns
    let added = &columns::list_columns(&target, summary.class_ids[0]).unwrap()[2..];
    assert_eq!(added[0].kind, ColumnKind::Round { source: Some(added[1].id), decimals: 1 });
}

/// An older backup is migrated first, so its rounding link across classes is dropped, and the class
/// it rounds keeps its decimals. The class is still added, with its columns.
#[test]
fn an_older_backup_with_a_link_across_classes_adds_without_that_link() {
    let fixture = Fixture::new();
    let file = version_ten_backup_across_classes(&fixture);
    let state = fixture.set_up();
    let opened = state.open_backup(&file, None).unwrap();
    let summary = state.add_backup_classes(opened).unwrap();
    let connection = state.connection().unwrap();
    let class = class_id(connection, "1A");
    assert!(summary.class_ids.contains(&class));
    let loaded = sheet::load_sheet(connection, class).unwrap();
    let rounded = loaded.columns.iter().find(|column| matches!(column.kind, ColumnKind::Round { .. })).unwrap();
    assert_eq!(rounded.kind, ColumnKind::Round { source: None, decimals: 2 }, "no link across classes, decimals kept");
}

#[test]
fn a_failure_adds_nothing() {
    let fixture = Fixture::new();
    let source = database(&fixture, "source.db");
    crate::common::sample_class(&source);
    let target = database(&fixture, "target.db");
    let here = classes::create_class(&target, "Here").unwrap();
    pupils::add_new_pupil(&target, here, "Ana Pérez", "").unwrap();
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
    let titles: Vec<&str> =
        loaded.columns.iter().filter(|column| !column.kind.is_name()).map(|column| column.title.as_str()).collect();
    assert_eq!(titles, ["Exam", "Final grade"]);
    assert_eq!(loaded.rows[0].pupil.first_name, "Ana");
    assert_eq!(loaded.rows[0].values, ["Ana", "", "7,5", ""], "the names first, then the marks");
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
                target.execute("INSERT INTO pupils (first_name) VALUES (?1)", [name]).unwrap();
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

#[test]
fn regression_two_backup_pupils_never_join_the_same_pupil_here() {
    // both backup Anas matched the one Ana here: in one class the second enrolment broke the
    // class's primary key and the whole add failed; across classes two pupils became one
    let fixture = Fixture::new();
    let target = database(&fixture, "target.db");
    let here = classes::create_class(&target, "Here").unwrap();
    let ana = pupils::add_new_pupil(&target, here, "Ana García", "").unwrap();

    let source = database(&fixture, "source.db");
    let x = classes::create_class(&source, "X").unwrap();
    let y = classes::create_class(&source, "Y").unwrap();
    pupils::add_new_pupil(&source, x, "Ana García", "").unwrap();
    pupils::add_new_pupil(&source, x, "Ana García", "").unwrap();
    pupils::add_new_pupil(&source, y, "Ana García", "").unwrap();

    let summary = backup::add_classes(&target, &source).unwrap();
    assert_eq!((summary.pupils, summary.existing_pupils), (3, 1));
    let ids = |class: &str| -> Vec<i64> {
        pupils::list_class_pupils(&target, class_id(&target, class))
            .unwrap()
            .into_iter()
            .map(|pupil| pupil.id)
            .collect()
    };
    let (in_x, in_y) = (ids("X"), ids("Y"));
    assert_eq!(in_x[0], ana, "the first one joins the Ana here");
    assert_ne!(in_x[1], ana);
    assert!(!in_x.contains(&in_y[0]), "the Ana of Y is a pupil of her own");
    assert_eq!(count(&target, "pupils"), 3);
}

#[test]
fn added_classes_keep_their_column_widths() {
    let fixture = Fixture::new();
    let source = database(&fixture, "source.db");
    let class = crate::common::sample_class(&source);
    let notes = columns::list_columns(&source, class).unwrap().last().unwrap().id;
    columns::set_column_width(&source, notes, Some(400)).unwrap();
    crate::common::set_name_widths(&source, class, 300, 180);
    let target = database(&fixture, "target.db");

    let summary = backup::add_classes(&target, &source).unwrap();
    let added = summary.class_ids[0];
    let widths: Vec<Option<u16>> = columns::list_columns(&target, added).unwrap().iter().map(|c| c.width).collect();
    let expected: Vec<Option<u16>> = columns::list_columns(&source, class).unwrap().iter().map(|c| c.width).collect();
    assert_eq!(widths, expected);
    assert_eq!(widths.last(), Some(&Some(400)));
    assert_eq!(crate::common::name_widths(&target, added), (Some(300), Some(180)), "the name columns' too");
}

#[test]
fn a_backup_pupil_joins_the_pupil_here_by_full_name() {
    let fixture = Fixture::new();
    let target = database(&fixture, "target.db");
    let here = classes::create_class(&target, "Here").unwrap();
    let ana = pupils::add_new_pupil(&target, here, "Ana", "Pérez").unwrap();
    let brais = pupils::add_new_pupil(&target, here, "Brais", "Otero").unwrap();

    let source = database(&fixture, "source.db");
    let x = classes::create_class(&source, "X").unwrap();
    pupils::add_new_pupil(&source, x, "ana", "PÉREZ").unwrap();
    // a backup pupil whose whole name is still in the first name is the same pupil
    pupils::add_new_pupil(&source, x, "Brais Otero", "").unwrap();
    pupils::add_new_pupil(&source, x, "Carla", "Iglesias").unwrap();

    let summary = backup::add_classes(&target, &source).unwrap();
    assert_eq!((summary.pupils, summary.existing_pupils), (3, 2));
    let added: Vec<(i64, String, String)> = pupils::list_class_pupils(&target, class_id(&target, "X"))
        .unwrap()
        .into_iter()
        .map(|pupil| (pupil.id, pupil.first_name, pupil.surname))
        .collect();
    assert_eq!(added[0], (ana, "Ana".to_owned(), "Pérez".to_owned()), "joined, under the names here");
    assert_eq!(added[1].0, brais);
    assert_eq!((added[2].1.as_str(), added[2].2.as_str()), ("Carla", "Iglesias"), "a new pupil keeps both names");
}

#[test]
fn an_added_class_keeps_its_column_order_with_the_names() {
    let fixture = Fixture::new();
    let source = database(&fixture, "source.db");
    let class = crate::common::sample_class(&source);
    let (_, surname) = crate::common::name_columns(&source, class);
    columns::move_column_to(&source, surname, 3).unwrap();
    crate::common::set_name_widths(&source, class, 200, 120);
    let target = database(&fixture, "target.db");
    let added = backup::add_classes(&target, &source).unwrap().class_ids[0];
    let kinds = |connection: &Connection, class: i64| -> Vec<String> {
        columns::list_columns(connection, class)
            .unwrap()
            .iter()
            .map(|column| format!("{:?}", column.kind.column_type()))
            .collect()
    };
    assert_eq!(kinds(&target, added), kinds(&source, class));
    assert_eq!(kinds(&target, added)[3], "Surname");
    assert_eq!(crate::common::name_widths(&target, added), (Some(200), Some(120)));
}
