// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:51:22.867116
//+ Revised: 	2026/11/25 11:43:14.759834
// -----------------------------------------------------------------------------

//! Pictures in the database: set, replaced, removed and deleted with their pupil; and the order the
//! pupils' numbers come from.

use crate::support::open;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::{classes, pictures, pupils};
use rusqlite::Connection;

fn with_ana() -> (tempfile::TempDir, Connection, i64, i64) {
    let (dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let ana = pupils::add_new_pupil(&connection, class, "Ana", "").unwrap();
    (dir, connection, class, ana)
}

#[test]
fn a_picture_is_set_replaced_and_removed() {
    let (_dir, connection, _, ana) = with_ana();
    assert_eq!(pictures::picture(&connection, ana).unwrap(), None);
    pictures::set_picture(&connection, ana, b"first").unwrap();
    pictures::set_picture(&connection, ana, b"second").unwrap();
    assert_eq!(pictures::picture(&connection, ana).unwrap(), Some(b"second".to_vec()));
    pictures::remove_picture(&connection, ana).unwrap();
    assert_eq!(pictures::picture(&connection, ana).unwrap(), None);
    assert!(matches!(pictures::remove_picture(&connection, ana), Err(AppError::NotEditable)), "there is none now");
}

#[test]
fn a_picture_needs_its_pupil() {
    let (_dir, connection, _, _) = with_ana();
    assert!(matches!(pictures::set_picture(&connection, 99, b"x"), Err(AppError::NotEditable)));
}

#[test]
fn a_classes_pictures_are_those_of_its_pupils_only() {
    let (_dir, connection, class, ana) = with_ana();
    let bea = pupils::add_new_pupil(&connection, class, "Bea", "").unwrap();
    let other = classes::create_class(&connection, "2B").unwrap();
    let carla = pupils::add_new_pupil(&connection, other, "Carla", "").unwrap();
    pictures::set_picture(&connection, ana, b"ana").unwrap();
    pictures::set_picture(&connection, carla, b"carla").unwrap();
    let here = pictures::class_pictures(&connection, class).unwrap();
    assert_eq!((here.len(), here.get(&ana)), (1, Some(&b"ana".to_vec())));
    assert!(!here.contains_key(&bea));
    pupils::add_existing_pupil(&connection, other, ana).unwrap();
    assert_eq!(pictures::class_pictures(&connection, other).unwrap().len(), 2, "shared like the pupil");
}

#[test]
fn a_deleted_pupils_picture_goes_with_them() {
    let (_dir, connection, class, ana) = with_ana();
    pictures::set_picture(&connection, ana, b"ana").unwrap();
    pupils::remove_from_class(&connection, class, ana).unwrap();
    assert!(pictures::picture(&connection, ana).unwrap().is_some(), "leaving a class keeps it");
    pupils::delete_pupil_in_no_class(&connection, ana).unwrap();
    let left: i64 = connection.query_row("SELECT COUNT(*) FROM pupil_pictures", [], |row| row.get(0)).unwrap();
    assert_eq!(left, 0);
}

#[test]
fn a_classes_list_is_by_surname() {
    let (_dir, connection, class, _) = with_ana();
    for (first_name, surname) in [("Uxía", "Fernández"), ("Brais", "Ótero"), ("Iria", "Ñáñez"), ("Noa", "Núñez")]
    {
        pupils::add_new_pupil(&connection, class, first_name, surname).unwrap();
    }
    let names: Vec<String> =
        pupils::class_list(&connection, class).unwrap().iter().map(|pupil| pupil.listed_name()).collect();
    assert_eq!(names, ["Ana", "Fernández, Uxía", "Núñez, Noa", "Ñáñez, Iria", "Ótero, Brais"]);
}
