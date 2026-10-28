// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:18.136173
//+ Revised: 	2026/10/28 15:53:09.581440
// -----------------------------------------------------------------------------

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use avalio_lite::error::{AppError, Folder, NumberField};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::i18n::{LANGUAGES, Language, Message, default_column_title, error_message, localize_number, text};

/// English language is the source text in the .slint files; every other language needs a .po file.
const TRANSLATED: [Language; 1] = [Language::Galician];

fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Every string passed to @tr(...) in the UI, with plural forms joined as "singular|plural".
fn ui_strings() -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for entry in fs::read_dir(manifest_path("ui")).unwrap() {
        let source = fs::read_to_string(entry.unwrap().path()).unwrap();
        for chunk in source.split("@tr(").skip(1) {
            let mut quoted = chunk.split('"');
            let singular = quoted.nth(1).unwrap().to_owned();
            let rest = quoted.next().unwrap_or_default();
            let key = if rest.trim() == "|" { format!("{singular}|{}", quoted.next().unwrap()) } else { singular };
            found.insert(key);
        }
    }
    found
}

/// The language's .po file, with Windows line endings (as a checkout may have) made plain.
fn read_po(language: Language) -> String {
    let path = manifest_path(&format!("lang/{}/LC_MESSAGES/avalio-lite.po", language.code()));
    fs::read_to_string(path).unwrap().replace('\r', "")
}

/// msgid -> whether every msgstr of that entry is filled in.
fn po_entries(language: Language) -> Vec<(String, bool)> {
    let source = read_po(language);
    let quoted = |line: &str| line.split_once('"').map(|(_, rest)| rest.trim_end().trim_end_matches('"').to_owned());
    let mut entries = Vec::new();
    for block in source.split("\n\n") {
        let lines: Vec<&str> = block.lines().filter(|line| !line.starts_with('#')).collect();
        let Some(id) = lines.iter().find(|line| line.starts_with("msgid ")).and_then(|line| quoted(line)) else {
            continue;
        };
        if id.is_empty() {
            continue; // header
        }
        let plural = lines.iter().find(|line| line.starts_with("msgid_plural ")).and_then(|line| quoted(line));
        let translated = lines
            .iter()
            .filter(|line| line.starts_with("msgstr"))
            .all(|line| quoted(line).is_some_and(|value| !value.is_empty()));
        let key = plural.map_or(id.clone(), |plural| format!("{id}|{plural}"));
        entries.push((key, translated));
    }
    entries
}

#[test]
fn every_ui_string_is_translated_and_no_stale_entries_remain() {
    let ui = ui_strings();
    assert!(ui.len() > 40, "the extractor found suspiciously few strings: {}", ui.len());
    for language in TRANSLATED {
        let entries = po_entries(language);
        let ids: BTreeSet<String> = entries.iter().map(|(id, _)| id.clone()).collect();
        let missing: Vec<_> = ui.difference(&ids).collect();
        let stale: Vec<_> = ids.difference(&ui).collect();
        let empty: Vec<_> = entries.iter().filter(|(_, translated)| !translated).map(|(id, _)| id).collect();
        assert!(missing.is_empty(), "{} is missing: {missing:#?}", language.code());
        assert!(stale.is_empty(), "{} has entries no longer in the UI: {stale:#?}", language.code());
        assert!(empty.is_empty(), "{} has untranslated entries: {empty:#?}", language.code());
        assert_eq!(entries.len(), ids.len(), "{} has duplicate entries", language.code());
    }
}

#[test]
fn placeholders_survive_translation() {
    for language in TRANSLATED {
        let source = read_po(language);
        for block in source.split("\n\n") {
            let count = |prefix: &str, placeholder: &str| {
                block
                    .lines()
                    .filter(|line| line.starts_with(prefix))
                    .map(|line| line.matches(placeholder).count())
                    .max()
            };
            for placeholder in ["{}", "{n}"] {
                if let Some(in_source) = count("msgid ", placeholder) {
                    assert_eq!(count("msgstr", placeholder), Some(in_source), "{placeholder} lost in:\n{block}");
                }
            }
        }
    }
}

#[test]
fn language_codes_round_trip() {
    for language in LANGUAGES {
        assert_eq!(Language::from_code(language.code()), Some(language));
        assert!(!language.native_name().is_empty());
    }
    assert_eq!(Language::from_code("fr"), None);
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

/// One of each message. The match below has no wildcard, so a new variant won't compile until
/// it is added here too, and then this test checks its translation.
fn every_message() -> Vec<Message<'static>> {
    let all = vec![
        Message::ConfirmDeleteClass { class: "1A" },
        Message::ConfirmRemovePupil { pupil: "Ana", class: "1A" },
        Message::ConfirmDeleteColumn { column: "Exam" },
        Message::DatabaseMoved { path: "D:\\x" },
        Message::LogsMoved { path: "D:\\y" },
        Message::DatabaseFileType,
        Message::WindowFailed { reason: "boom" },
        Message::LockNeedsPin,
        Message::PinTurnedOn,
        Message::PinChanged,
        Message::PinTurnedOff,
    ];
    for message in &all {
        match message {
            Message::ConfirmDeleteClass { .. }
            | Message::ConfirmRemovePupil { .. }
            | Message::ConfirmDeleteColumn { .. }
            | Message::DatabaseMoved { .. }
            | Message::LogsMoved { .. }
            | Message::DatabaseFileType
            | Message::WindowFailed { .. }
            | Message::LockNeedsPin
            | Message::PinTurnedOn
            | Message::PinChanged
            | Message::PinTurnedOff => {}
        }
    }
    all
}

/// One of each error, kept complete the same way as `every_message`.
fn every_error() -> Vec<AppError> {
    let parse_error = toml::from_str::<toml::Table>("not = [valid").unwrap_err();
    let all = vec![
        AppError::Io { path: "D:\\x".into(), source: std::io::Error::other("denied") },
        AppError::Database(rusqlite::Error::InvalidQuery),
        AppError::ConfigParse { path: "D:\\x".into(), source: parse_error },
        AppError::ConfigSerialize(serde::ser::Error::custom("denied")),
        AppError::NoConfigDir,
        AppError::EmptyPath(Folder::Database),
        AppError::EmptyPath(Folder::Logs),
        AppError::NotFound("D:\\x".into()),
        AppError::AlreadyExists("D:\\x".into()),
        AppError::NewerSchema { found: 3, supported: 1 },
        AppError::IntegrityCheckFailed("D:\\x".into()),
        AppError::NoDatabaseOpen,
        AppError::EmptyName,
        AppError::DuplicateClassName("1A".into()),
        AppError::InvalidNumber { field: NumberField::Weight, value: "x".into() },
        AppError::InvalidNumber { field: NumberField::MaxScore, value: "x".into() },
        AppError::InvalidNumber { field: NumberField::Decimals, value: "x".into() },
        AppError::NotEditable,
        AppError::InvalidRoundSource,
        AppError::InvalidPin,
        AppError::PinMismatch,
        AppError::WrongPin,
        AppError::PinWait { seconds: 3 },
    ];
    for error in &all {
        match error {
            AppError::Io { .. }
            | AppError::Database(_)
            | AppError::ConfigParse { .. }
            | AppError::ConfigSerialize(_)
            | AppError::NoConfigDir
            | AppError::EmptyPath(_)
            | AppError::NotFound(_)
            | AppError::AlreadyExists(_)
            | AppError::NewerSchema { .. }
            | AppError::IntegrityCheckFailed(_)
            | AppError::NoDatabaseOpen
            | AppError::EmptyName
            | AppError::DuplicateClassName(_)
            | AppError::InvalidNumber { .. }
            | AppError::NotEditable
            | AppError::InvalidRoundSource
            | AppError::InvalidPin
            | AppError::PinMismatch
            | AppError::WrongPin
            | AppError::PinWait { .. } => {}
        }
    }
    all
}

#[test]
fn rust_messages_are_translated_and_keep_their_details() {
    for message in every_message() {
        let (english, galician) = (text(Language::English, &message), text(Language::Galician, &message));
        assert_ne!(english, galician, "untranslated: {message:?}");
        for detail in ["1A", "Ana", "Exam", "D:\\x", "D:\\y", "boom"] {
            assert_eq!(english.contains(detail), galician.contains(detail), "{detail} lost in {galician}");
        }
    }
}

#[test]
fn errors_are_translated_and_keep_their_details() {
    for error in every_error() {
        let (english, galician) = (error_message(Language::English, &error), error_message(Language::Galician, &error));
        assert_ne!(english, galician, "untranslated: {error:?}");
        for detail in ["1A", "D:\\x", "denied", "\"x\"", "3"] {
            // Galician quotes with « » where English uses " "
            let galician_form =
                detail.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')).map(|inner| format!("«{inner}»"));
            let in_galician = galician.contains(detail) || galician_form.is_some_and(|form| galician.contains(&form));
            assert_eq!(english.contains(detail), in_galician, "{detail} lost in {galician}");
        }
    }
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
    }
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
