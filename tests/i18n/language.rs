// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.493117
//+ Revised: 	2026/11/23 00:26:09.493117
// -----------------------------------------------------------------------------

//! The language itself: its codes and names, the system's locale, numbers, and the words of the
//! columns, exports, shortcuts and file dialogs in each language.

use avalio_lite::gradebook::model::{ColumnTitles, ColumnType};
use avalio_lite::i18n::shortcuts::{action_label, display, key_names};
use avalio_lite::i18n::{
    LANGUAGES, Language, column_type_name, current, default_column_title, export_labels, localize_number, set_current,
};
use avalio_lite::shortcuts::{Action, KeyCombo};

#[test]
fn language_codes_round_trip() {
    for language in LANGUAGES {
        assert_eq!(Language::from_code(language.code()), Some(language));
    }
    assert_eq!(Language::from_code("fr"), None);
}

/// Kills the mutant that makes `native_name` return a made-up name.
#[test]
fn each_language_is_named_in_itself() {
    assert_eq!(Language::English.native_name(), "English");
    assert_eq!(Language::Galician.native_name(), "Galego");
}

/// The only test in this binary that touches the process-wide language, so none races it. Kills
/// the mutants of `current`, `set_current` and `index` (a fixed index, a default language, == as !=).
#[test]
fn the_current_language_is_the_last_one_set() {
    for language in [Language::Galician, Language::English, Language::Galician] {
        set_current(language);
        assert_eq!(current(), language);
    }
    set_current(Language::default());
}

/// Kills the mutant that makes `from_system` return the default language, wherever Windows isn't
/// set to English.
#[test]
fn the_system_language_follows_the_windows_locale() {
    assert_eq!(Language::from_system(), Language::from_locale(sys_locale::get_locale().as_deref()));
}

#[test]
fn windows_locale_picks_galician_only_for_galician() {
    assert_eq!(Language::from_locale(Some("gl-ES")), Language::Galician);
    assert_eq!(Language::from_locale(Some("GL")), Language::Galician);
    assert_eq!(Language::from_locale(Some("es-ES")), Language::English);
    assert_eq!(Language::from_locale(Some("en-GB")), Language::English);
    assert_eq!(Language::from_locale(None), Language::English);
}

#[test]
fn galician_numbers_use_a_decimal_comma() {
    assert_eq!(localize_number(Language::Galician, "7.65"), "7,65");
    assert_eq!(localize_number(Language::English, "7.65"), "7.65");
    assert_eq!(localize_number(Language::Galician, "8"), "8");
}

#[test]
fn default_column_titles_exist_in_every_language() {
    for language in LANGUAGES {
        for column_type in
            [ColumnType::Info, ColumnType::Test, ColumnType::Weighted, ColumnType::Average, ColumnType::Round]
        {
            assert!(!default_column_title(language, column_type, 1).is_empty());
        }
        assert!(default_column_title(language, ColumnType::Test, 3).ends_with('3'));
        // a name column's title is its header
        for name in [ColumnType::FirstName, ColumnType::Surname] {
            assert_eq!(default_column_title(language, name, 1), column_type_name(language, name));
        }
    }
}

/// The file dialogs filter by kind: a spreadsheet opens from any program, but is saved as .xlsx.
#[test]
fn file_kinds_have_extensions_and_translated_filters() {
    use avalio_lite::window::dialogs::FileKind;
    assert_eq!(FileKind::Spreadsheet.open_extensions(), ["xlsx", "xls", "ods"]);
    assert_eq!(FileKind::Spreadsheet.save_extensions(), ["xlsx"]);
    assert_eq!(FileKind::Database.open_extensions(), ["db"]);
    assert_eq!(FileKind::Backup.save_extensions(), ["avalio"]);
    for kind in [FileKind::Database, FileKind::Spreadsheet, FileKind::Backup] {
        let (english, galician) = (kind.filter_name(Language::English), kind.filter_name(Language::Galician));
        assert!(!english.is_empty() && english != galician, "{kind:?}: {english} / {galician}");
    }
    assert_eq!(FileKind::Spreadsheet.filter_name(Language::Galician), "Libro de Excel");
}

#[test]
fn export_words_are_translated() {
    let (english, galician) = (export_labels(Language::English), export_labels(Language::Galician));
    assert_ne!(english.first_name, galician.first_name);
    assert_ne!(english.surname, galician.surname);
    assert_eq!(
        avalio_lite::i18n::surname_headers().collect::<Vec<_>>(),
        [english.surname, galician.surname],
        "an import knows the surname header of every language"
    );
    assert_ne!(english.sheet, galician.sheet);
    assert_ne!(english.columns_tab, galician.columns_tab);
    assert_ne!(english.yes, galician.yes);
    for (english, galician) in english.column_headers.iter().zip(galician.column_headers) {
        assert_ne!(*english, galician);
    }
    for column_type in
        [ColumnType::Info, ColumnType::Test, ColumnType::Weighted, ColumnType::Average, ColumnType::Round]
    {
        assert_ne!(column_type_name(Language::English, column_type), column_type_name(Language::Galician, column_type));
    }
}

#[test]
fn every_action_has_a_label_in_every_language() {
    for language in LANGUAGES {
        for action in Action::ALL {
            assert!(!action_label(language, action).is_empty(), "{action:?} in {language:?}");
        }
    }
    assert_ne!(action_label(Language::English, Action::Lock), action_label(Language::Galician, Action::Lock));
}

#[test]
fn shift_is_maius_in_galician() {
    let combo: KeyCombo = "Ctrl+Shift+D".parse().unwrap();
    assert_eq!(key_names(Language::English, &combo), ["Ctrl", "Shift", "D"]);
    assert_eq!(key_names(Language::Galician, &combo), ["Ctrl", "Maiús", "D"]);
    assert_eq!(display(Language::Galician, &combo), "Ctrl+Maiús+D");
}

proptest::proptest! {
    #[test]
    fn a_localized_number_reads_back_as_the_same_value(value in -1000.0f64..1000.0) {
        let formatted = avalio_lite::gradebook::compute::format_number(value);
        for language in LANGUAGES {
            let shown = localize_number(language, &formatted);
            proptest::prop_assert_eq!(
                avalio_lite::gradebook::compute::parse_number(&shown),
                avalio_lite::gradebook::compute::parse_number(&formatted)
            );
        }
    }
}

#[test]
fn language_names_columns_like_default_column_title() {
    let types = [ColumnType::Info, ColumnType::Test, ColumnType::Weighted, ColumnType::Average, ColumnType::Round];
    for language in [Language::English, Language::Galician] {
        for column_type in types {
            for number in 1..=3 {
                assert_eq!(
                    language.default_title(column_type, number),
                    default_column_title(language, column_type, number),
                    "{language:?} {column_type:?} {number}"
                );
            }
        }
    }
}
