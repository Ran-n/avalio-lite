// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.625518
//+ Revised: 	2026/11/24 16:26:03.246907
// -----------------------------------------------------------------------------

//! Names: sorting and matching them in any alphabet, and first names with surnames.

use crate::support::open;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::{classes, pupils};

#[test]
fn regression_accented_names_sort_with_their_letter() {
    // SQLite's NOCASE folds only A to Z: "Álvaro" and "Ánxela" were listed after "Zoe"
    let (_dir, connection) = open();
    for name in ["Zoo", "árabe", "Ñu", "Nube", "Oca", "Abeto"] {
        classes::create_class(&connection, name).unwrap();
    }
    let names: Vec<String> = classes::list_classes(&connection).unwrap().into_iter().map(|class| class.name).collect();
    assert_eq!(names, ["Abeto", "árabe", "Nube", "Ñu", "Oca", "Zoo"]);

    let other = classes::create_class(&connection, "Other").unwrap();
    let here = classes::create_class(&connection, "Here").unwrap();
    for name in ["Zoe", "Ánxela", "brais", "Álvaro"] {
        pupils::add_new_pupil(&connection, other, name, "").unwrap();
    }
    let candidates: Vec<String> =
        pupils::pupils_not_in_class(&connection, here).unwrap().into_iter().map(|pupil| pupil.first_name).collect();
    assert_eq!(candidates, ["Álvaro", "Ánxela", "brais", "Zoe"]);
}

#[test]
fn regression_names_match_ignoring_case_in_any_alphabet() {
    // "álvaro" didn't find "Álvaro": NOCASE leaves Á and á apart
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let alvaro = pupils::add_new_pupil(&connection, class, "Álvaro Núñez", "").unwrap();
    pupils::add_new_pupil(&connection, class, "Alvaro Nunez", "").unwrap();
    let found: Vec<i64> =
        pupils::pupils_named(&connection, "  ÁLVARO NÚÑEZ ").unwrap().into_iter().map(|pupil| pupil.id).collect();
    assert_eq!(found, [alvaro], "case is ignored, accents are not");
}

mod name_keys {
    use avalio_lite::gradebook::model::{name_match_key, name_sort_key};
    use proptest::prelude::*;

    #[test]
    fn accents_fold_and_n_tilde_stays_after_n() {
        assert_eq!(name_sort_key("Ánxela"), name_sort_key("anxela"));
        assert_eq!(name_sort_key("A\u{301}lvaro"), name_sort_key("Álvaro"), "an accent typed as a mark of its own");
        assert_eq!(name_sort_key("n\u{303}u"), name_sort_key("ñu"));
        assert_eq!(name_sort_key("a\u{303}"), name_sort_key("ã"), "a tilde makes a letter of its own only of n");
        assert_eq!(name_sort_key("Ýago"), name_sort_key("yago"));
        assert!(name_sort_key("nz") < name_sort_key("ña") && name_sort_key("ña") < name_sort_key("o"));
    }

    proptest! {
        #[test]
        fn case_never_changes_the_order_or_the_match(name in "[a-zA-Zá-úÁ-Úñ ]{0,20}") {
            prop_assert_eq!(name_sort_key(&name.to_uppercase()), name_sort_key(&name.to_lowercase()));
            prop_assert_eq!(name_match_key(&name.to_uppercase()), name_match_key(&name.to_lowercase()));
            prop_assert_eq!(name_match_key(&format!("  {name} ")), name_match_key(&name));
        }
    }
}

mod decomposed_accents {
    use avalio_lite::gradebook::model::name_match_key;

    #[test]
    fn regression_an_accent_typed_as_a_mark_matches_the_same_name() {
        // "A" + U+0301 (as some spreadsheets store it) didn't match a stored "Álvaro", and an
        // import or backup made a second pupil
        assert_eq!(
            name_match_key("A\u{301}lvaro Nun\u{303}ez"),
            name_match_key("Álvaro Núñez".replace('ú', "u").as_str())
        );
        assert_eq!(name_match_key("A\u{301}lvaro"), name_match_key("álvaro"));
        assert_eq!(name_match_key("Franc\u{327}ois"), name_match_key("françois"));
        assert_ne!(name_match_key("Alvaro"), name_match_key("Álvaro"), "accents still count");
        assert_eq!(name_match_key("x\u{301}"), "x\u{301}", "a mark no letter takes stays as it is");
        // every accent the table knows, against the same letter typed in one piece
        for (marked, whole) in [
            ("e\u{300}", "è"),
            ("o\u{302}", "ô"),
            ("u\u{308}", "ü"),
            ("y\u{308}", "ÿ"),
            ("a\u{303}", "ã"),
            ("o\u{303}", "õ"),
            ("n\u{303}", "ñ"),
            ("c\u{327}", "ç"),
        ] {
            assert_eq!(name_match_key(marked), whole, "{whole}");
        }
        for unmatched in ["y\u{300}", "e\u{303}", "a\u{327}", "a\u{30A}"] {
            assert_eq!(name_match_key(unmatched), unmatched, "{unmatched:?} has no letter of its own here");
        }
    }
}

#[test]
fn pupils_have_a_first_name_and_an_optional_surname() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    pupils::add_new_pupil(&connection, class, "  Ana ", " Pérez López  ").unwrap();
    pupils::add_new_pupil(&connection, class, "Brais", "").unwrap();
    assert!(matches!(pupils::add_new_pupil(&connection, class, "", "Otero"), Err(AppError::EmptyName)));
    assert!(matches!(pupils::add_new_pupil(&connection, class, "   ", "Otero"), Err(AppError::EmptyName)));
    let stored: Vec<(String, String)> = pupils::list_class_pupils(&connection, class)
        .unwrap()
        .into_iter()
        .map(|pupil| (pupil.first_name, pupil.surname))
        .collect();
    assert_eq!(
        stored,
        [("Ana".to_owned(), "Pérez López".to_owned()), ("Brais".to_owned(), String::new())],
        "both trimmed, and nothing added when the first name is missing"
    );
}

#[test]
fn first_name_and_surname_are_renamed_separately_everywhere() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana", "Pérez").unwrap();
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    pupils::set_first_name(&connection, ana, " Ana María ").unwrap();
    pupils::set_surname(&connection, ana, " Pérez López ").unwrap();
    for class in [a, b] {
        let pupil = &pupils::list_class_pupils(&connection, class).unwrap()[0];
        assert_eq!((pupil.first_name.as_str(), pupil.surname.as_str()), ("Ana María", "Pérez López"));
    }
    assert!(matches!(pupils::set_first_name(&connection, ana, "  "), Err(AppError::EmptyName)));
    pupils::set_surname(&connection, ana, "  ").unwrap();
    let pupil = pupils::pupil_name(&connection, ana).unwrap();
    assert_eq!((pupil.first_name.as_str(), pupil.surname.as_str()), ("Ana María", ""), "a blank surname clears it");
    assert!(matches!(pupils::set_surname(&connection, 9999, "x"), Err(AppError::NotEditable)));
}

#[test]
fn pupil_menus_sort_by_surname_then_first_name() {
    let (_dir, connection) = open();
    let other = classes::create_class(&connection, "Other").unwrap();
    let here = classes::create_class(&connection, "Here").unwrap();
    for (first_name, surname) in
        [("Ana", "Rodríguez"), ("Bea", "Pérez"), ("Álvaro", ""), ("Ana", "Pérez"), ("Xoán", "Alonso")]
    {
        pupils::add_new_pupil(&connection, other, first_name, surname).unwrap();
    }
    let listed: Vec<String> =
        pupils::pupils_not_in_class(&connection, here).unwrap().iter().map(|pupil| pupil.listed_name()).collect();
    assert_eq!(listed, ["Alonso, Xoán", "Álvaro", "Pérez, Ana", "Pérez, Bea", "Rodríguez, Ana"]);
    classes::delete_class(&connection, other).unwrap();
    let in_no_class: Vec<String> =
        pupils::pupils_in_no_class(&connection).unwrap().iter().map(|pupil| pupil.listed_name()).collect();
    assert_eq!(in_no_class, listed, "pupils in no class are listed the same way");
}

#[test]
fn full_names_match_whichever_field_holds_the_words() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let split = pupils::add_new_pupil(&connection, class, "Álvaro", "Núñez").unwrap();
    let whole = pupils::add_new_pupil(&connection, class, "Álvaro Núñez", "").unwrap();
    let found: Vec<i64> =
        pupils::pupils_named(&connection, " álvaro NÚÑEZ ").unwrap().into_iter().map(|pupil| pupil.id).collect();
    assert_eq!(found, [split, whole], "the full name is compared, ignoring case");
}
