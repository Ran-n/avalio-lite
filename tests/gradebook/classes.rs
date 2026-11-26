// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.095449
//+ Revised: 	2026/11/23 13:00:35.095449
// -----------------------------------------------------------------------------

//! Classes and pupils: listing, names, deleting, sharing pupils between classes.

use crate::support::open;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::{classes, pupils};

#[test]
fn classes_are_listed_by_name_with_pupil_counts() {
    let (_dir, connection) = open();
    let b = classes::create_class(&connection, " 2B ").unwrap();
    let a = classes::create_class(&connection, "1A").unwrap();
    pupils::add_new_pupil(&connection, b, "Ana", "").unwrap();
    let listed = classes::list_classes(&connection).unwrap();
    assert_eq!(
        listed.iter().map(|class| (class.id, class.name.as_str(), class.pupil_count)).collect::<Vec<_>>(),
        [(a, "1A", 0), (b, "2B", 1)]
    );
}

#[test]
fn class_names_must_be_unique_and_non_empty() {
    let (_dir, connection) = open();
    let id = classes::create_class(&connection, "1A").unwrap();
    assert!(matches!(classes::create_class(&connection, "1A"), Err(AppError::DuplicateClassName(_))));
    assert!(matches!(classes::create_class(&connection, "   "), Err(AppError::EmptyName)));
    let other = classes::create_class(&connection, "2B").unwrap();
    assert!(matches!(classes::rename_class(&connection, other, "1A"), Err(AppError::DuplicateClassName(_))));
    classes::rename_class(&connection, id, "1A bis").unwrap();
    assert_eq!(classes::class_name(&connection, id).unwrap(), "1A bis");
}

#[test]
fn adjacent_class_wraps_around() {
    use classes::adjacent_index;
    assert_eq!(adjacent_index(0, None, 1), None, "no classes, nothing to switch to");
    assert_eq!(adjacent_index(0, None, -1), None);
    assert_eq!(adjacent_index(1, Some(0), 1), Some(0), "one class stays selected");
    assert_eq!(adjacent_index(1, Some(0), -1), Some(0));
    assert_eq!(adjacent_index(3, Some(0), 1), Some(1));
    assert_eq!(adjacent_index(3, Some(2), 1), Some(0), "past the last goes round to the first");
    assert_eq!(adjacent_index(3, Some(0), -1), Some(2), "before the first goes round to the last");
    assert_eq!(adjacent_index(3, Some(1), -1), Some(0));
    assert_eq!(adjacent_index(3, None, 1), Some(0), "with none selected, next is the first");
    assert_eq!(adjacent_index(3, None, -1), Some(2), "and previous the last");
}

#[test]
fn deleting_a_class_keeps_its_pupils_in_other_classes() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana", "").unwrap();
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    classes::delete_class(&connection, a).unwrap();
    assert_eq!(pupils::list_class_pupils(&connection, b).unwrap()[0].first_name, "Ana");
}

#[test]
fn pupils_are_shared_and_renamed_everywhere() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana", "").unwrap();
    assert_eq!(pupils::pupils_not_in_class(&connection, b).unwrap()[0].id, ana);
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    assert!(pupils::pupils_not_in_class(&connection, b).unwrap().is_empty());
    assert!(pupils::add_existing_pupil(&connection, b, ana).is_err(), "can't be added twice");
    pupils::set_first_name(&connection, ana, "Ana Pérez").unwrap();
    assert_eq!(pupils::list_class_pupils(&connection, a).unwrap()[0].first_name, "Ana Pérez");
    assert!(matches!(pupils::set_first_name(&connection, ana, " "), Err(AppError::EmptyName)));
}

#[test]
fn pupils_keep_the_order_they_were_added_in() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    for name in ["Xoán", "Ana", "Brais"] {
        pupils::add_new_pupil(&connection, class_id, name, "").unwrap();
    }
    let names: Vec<String> =
        pupils::list_class_pupils(&connection, class_id).unwrap().into_iter().map(|pupil| pupil.first_name).collect();
    assert_eq!(names, ["Xoán", "Ana", "Brais"]);
}

#[test]
fn renaming_a_class_to_another_classes_name_is_refused() {
    let (_dir, connection) = open();
    classes::create_class(&connection, "1A").unwrap();
    let second = classes::create_class(&connection, "2B").unwrap();
    let result = classes::rename_class(&connection, second, "1A");
    assert!(matches!(result, Err(AppError::DuplicateClassName(ref name)) if name == "1A"), "{result:?}");
}

#[test]
fn regression_renaming_something_that_no_longer_exists_is_reported() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let pupil = pupils::add_new_pupil(&connection, class, "Ana", "").unwrap();
    classes::delete_class(&connection, class).unwrap();
    connection.execute("DELETE FROM pupils WHERE id = ?1", [pupil]).unwrap();
    assert!(matches!(classes::rename_class(&connection, class, "2B"), Err(AppError::NotEditable)));
    assert!(matches!(pupils::set_first_name(&connection, pupil, "Bea"), Err(AppError::NotEditable)));
}

#[test]
fn a_pupil_in_no_class_can_be_deleted_and_only_then() {
    let (_dir, connection) = open();
    let a = classes::create_class(&connection, "1A").unwrap();
    let b = classes::create_class(&connection, "1B").unwrap();
    let ana = pupils::add_new_pupil(&connection, a, "Ana", "").unwrap();
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    let brais = pupils::add_new_pupil(&connection, b, "Brais", "").unwrap();
    assert!(pupils::pupils_in_no_class(&connection).unwrap().is_empty());

    pupils::remove_from_class(&connection, a, ana).unwrap();
    assert!(matches!(pupils::delete_pupil_in_no_class(&connection, ana), Err(AppError::NotEditable)), "still in 1B");
    // deleting a class leaves its pupils in no class, but kept
    classes::delete_class(&connection, b).unwrap();
    let left: Vec<String> =
        pupils::pupils_in_no_class(&connection).unwrap().into_iter().map(|pupil| pupil.first_name).collect();
    assert_eq!(left, ["Ana", "Brais"]);

    pupils::delete_pupil_in_no_class(&connection, ana).unwrap();
    assert!(pupils::pupils_named(&connection, "Ana").unwrap().is_empty());
    assert!(matches!(pupils::delete_pupil_in_no_class(&connection, ana), Err(AppError::NotEditable)), "already gone");
    assert_eq!(pupils::pupils_in_no_class(&connection).unwrap()[0].id, brais);
}
