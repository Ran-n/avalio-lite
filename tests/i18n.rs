// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:18.136173
//+ Revised: 	2026/11/02 08:27:51.228250
// -----------------------------------------------------------------------------

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use avalio_lite::error::{AppError, Folder, NumberField};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::i18n::shortcuts::{action_label, display, key_names};
use avalio_lite::i18n::{
    LANGUAGES, Language, Message, column_type_name, current, default_column_title, error_message, export_labels,
    localize_number, set_current, text,
};
use avalio_lite::shortcuts::{Action, KeyCombo, ProblemKind, ShortcutError, ShortcutProblem};

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

/// One of each message. The match below has no wildcard, so a new variant won't compile until
/// it is added here too, and then this test checks its translation.
fn every_message() -> Vec<Message<'static>> {
    let renamed: &'static [(String, String)] = Box::leak(Box::new([("1A".to_owned(), "1A (2)".to_owned())]));
    let all = vec![
        Message::ConfirmDeleteClass { class: "1A" },
        Message::ConfirmRemovePupil { pupil: "Ana", class: "1A" },
        Message::ConfirmDeleteColumn { column: "Exam" },
        Message::ConfirmDeletePupil { pupil: "Ana" },
        Message::DatabaseMoved { path: "D:\\x" },
        Message::LogsMoved { path: "D:\\y" },
        Message::DatabaseFileType,
        Message::BackupFileType,
        Message::BackupExported { path: "D:\\x" },
        Message::ConfirmRestore { file: "Exam", classes: 4, pupils: 1 },
        Message::BackupRestored { file: "Exam" },
        Message::BackupClassesAdded { classes: 3, pupils: 1, existing: 2, file: "Exam" },
        Message::BackupClassesAdded { classes: 1, pupils: 2, existing: 1, file: "Exam" },
        Message::BackupClassesAdded { classes: 2, pupils: 2, existing: 0, file: "Exam" },
        Message::BackupClassesRenamed { pairs: renamed },
        Message::BackupHasNoClasses,
        Message::WindowFailed,
        Message::LockNeedsPin,
        Message::PinTurnedOn,
        Message::PinChanged,
        Message::PinTurnedOff,
        Message::ShortcutChanged { action: "Exam", keys: "boom" },
        Message::ShortcutsReset,
        Message::SheetExported { class: "1A", path: "D:\\x" },
        Message::SpreadsheetFileType,
        Message::ExportWeightsWarning { total: "boom" },
        Message::ConfirmImportOverwrite { count: 2, class: "1A" },
        Message::ImportDone { pupils: 2, new_pupils: 1, marks: 3, class: "1A" },
        Message::ImportLeftOut { headers: "Phone", count: 1 },
        Message::ImportErrorCells { count: 2 },
        Message::UntitledColumn { letter: "D" },
    ];
    for message in &all {
        match message {
            Message::ConfirmDeleteClass { .. }
            | Message::ConfirmRemovePupil { .. }
            | Message::ConfirmDeleteColumn { .. }
            | Message::ConfirmDeletePupil { .. }
            | Message::DatabaseMoved { .. }
            | Message::LogsMoved { .. }
            | Message::DatabaseFileType
            | Message::BackupFileType
            | Message::BackupExported { .. }
            | Message::ConfirmRestore { .. }
            | Message::BackupRestored { .. }
            | Message::BackupClassesAdded { .. }
            | Message::BackupClassesRenamed { .. }
            | Message::BackupHasNoClasses
            | Message::WindowFailed
            | Message::LockNeedsPin
            | Message::PinTurnedOn
            | Message::PinChanged
            | Message::PinTurnedOff
            | Message::ShortcutChanged { .. }
            | Message::ShortcutsReset
            | Message::SheetExported { .. }
            | Message::SpreadsheetFileType
            | Message::ExportWeightsWarning { .. }
            | Message::ConfirmImportOverwrite { .. }
            | Message::ImportDone { .. }
            | Message::ImportLeftOut { .. }
            | Message::ImportErrorCells { .. }
            | Message::UntitledColumn { .. } => {}
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
        AppError::NotADatabase("D:\\x".into()),
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
        AppError::InvalidShortcuts(vec![ShortcutProblem {
            entry: "lock = \"x\"".into(),
            kind: ProblemKind::BadKeys(ShortcutError::UnknownKey),
        }]),
        AppError::Shortcut(ShortcutError::UnknownKey),
        AppError::Shortcut(ShortcutError::NeedsCtrlOrAlt),
        AppError::Shortcut(ShortcutError::CtrlAlt),
        AppError::Shortcut(ShortcutError::EditingKey('C')),
        AppError::Shortcut(ShortcutError::Clash { keys: "Ctrl+H".parse().unwrap(), taken_by: Action::Lock }),
        AppError::SpreadsheetWrite { path: "D:\\x".into(), reason: "denied".into() },
        AppError::SpreadsheetRead { path: "D:\\x".into(), reason: "not a workbook".into() },
        AppError::NoPupilsInFile("D:\\x".into()),
        AppError::DuplicatePupilInFile("Ana".into()),
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
            | AppError::NotADatabase(_)
            | AppError::KeptAside { .. }
            | AppError::NoDatabaseOpen
            | AppError::EmptyName
            | AppError::DuplicateClassName(_)
            | AppError::InvalidNumber { .. }
            | AppError::NotEditable
            | AppError::InvalidRoundSource
            | AppError::InvalidPin
            | AppError::PinMismatch
            | AppError::WrongPin
            | AppError::PinWait { .. }
            | AppError::InvalidShortcuts(_)
            | AppError::Shortcut(_)
            | AppError::SpreadsheetWrite { .. }
            | AppError::SpreadsheetRead { .. }
            | AppError::NoPupilsInFile(_)
            | AppError::DuplicatePupilInFile(_) => {}
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

/// One of a thing is singular, and so is its verb in Galician; none or several are plural.
#[test]
fn import_messages_count_in_every_language() {
    let english = |message: &Message| text(Language::English, message);
    let galician = |message: &Message| text(Language::Galician, message);
    let done = |pupils, new_pupils, marks| Message::ImportDone { pupils, new_pupils, marks, class: "1A" };
    assert_eq!(english(&done(1, 0, 1)), "Imported 1 pupil and 1 mark or note into 1A.");
    assert_eq!(english(&done(3, 2, 0)), "Imported 3 pupils (2 new) and 0 marks or notes into 1A.");
    assert_eq!(galician(&done(1, 1, 1)), "Importouse 1 alumno (1 novo) e 1 nota ou observación en 1A.");
    assert_eq!(galician(&done(3, 2, 0)), "Importáronse 3 alumnos (2 novos) e 0 notas ou observacións en 1A.");
    assert_eq!(galician(&done(2, 0, 2)), "Importáronse 2 alumnos e 2 notas ou observacións en 1A.");
    let added = |classes, pupils| Message::BackupClassesAdded { classes, pupils, existing: 0, file: "Exam" };
    assert_eq!(galician(&added(1, 3)), "Engadiuse 1 clase e 3 alumnos desde Exam.");
    assert_eq!(galician(&added(2, 1)), "Engadíronse 2 clases e 1 alumno desde Exam.");
    assert_eq!(english(&added(1, 1)), "Added 1 class and 1 pupil from Exam.");
    let overwrite = |count| Message::ConfirmImportOverwrite { count, class: "1A" };
    assert_eq!(english(&overwrite(1)), "Importing replaces 1 mark or note already in 1A. Continue?");
    assert_eq!(galician(&overwrite(1)), "Ao importar substitúese 1 nota ou observación que xa hai en 1A. Continuar?");
    assert_eq!(
        galician(&overwrite(12)),
        "Ao importar substitúense 12 notas ou observacións que xa hai en 1A. Continuar?"
    );
    let left_out = |headers, count| Message::ImportLeftOut { headers, count };
    assert_eq!(galician(&left_out("Teléfono", 1)), "Quedou fóra: Teléfono.");
    assert_eq!(
        galician(&left_out("Teléfono, columna D (sen título)", 2)),
        "Quedaron fóra: Teléfono, columna D (sen título)."
    );
    assert_eq!(english(&left_out("Phone, column D (no title)", 2)), "Left out: Phone, column D (no title).");
    let errors = |count| Message::ImportErrorCells { count };
    assert_eq!(english(&errors(1)), "1 cell with an Excel error was skipped.");
    assert_eq!(english(&errors(2)), "2 cells with Excel errors were skipped.");
    assert_eq!(galician(&errors(1)), "Saltouse 1 cela cun erro de Excel.");
    assert_eq!(galician(&errors(2)), "Saltáronse 2 celas con erros de Excel.");
}

/// The file dialogs filter by kind: a spreadsheet opens from any program, but is saved as .xlsx.
#[test]
fn file_kinds_have_extensions_and_translated_filters() {
    use avalio_lite::app::dialogs::FileKind;
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
    assert_ne!(english.pupil, galician.pupil);
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

/// The system's own error text (English, technical) never reaches the user, in either language;
/// what went wrong is worded by its kind, and the path stays.
#[test]
fn regression_technical_error_text_is_replaced_by_friendly_sentences() {
    use std::io::{Error, ErrorKind};
    let raw = "RAW-SYSTEM-TEXT";
    let path = || std::path::PathBuf::from("D:\\x");
    let io = |kind| AppError::Io { path: path(), source: Error::new(kind, raw) };
    let sqlite = |code| {
        AppError::Database(rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), Some(raw.to_owned())))
    };
    let toml_error = toml::from_str::<toml::Value>("not = [valid").unwrap_err();
    let errors = [
        io(ErrorKind::NotFound),
        io(ErrorKind::PermissionDenied),
        io(ErrorKind::StorageFull),
        io(ErrorKind::Other),
        AppError::Io { path: path(), source: Error::from_raw_os_error(32) },
        sqlite(5),
        sqlite(6),
        sqlite(26),
        sqlite(11),
        sqlite(13),
        sqlite(1),
        AppError::ConfigParse { path: path(), source: toml_error },
        AppError::SpreadsheetWrite { path: path(), reason: raw.into() },
        AppError::SpreadsheetRead { path: path(), reason: raw.into() },
        AppError::SpreadsheetRead { path: path(), reason: "the reader panicked".into() },
    ];
    for error in &errors {
        for language in LANGUAGES {
            let shown = error_message(language, error);
            for technical in [raw, "os error", "SQLite", "TOML", "expected", "panicked", "code:"] {
                assert!(!shown.contains(technical), "{technical:?} reaches the user in {language:?}: {shown}");
            }
        }
    }
    let english = |error: &AppError| error_message(Language::English, error);
    assert!(english(&errors[0]).starts_with("Couldn't find D:\\x"));
    assert!(english(&errors[1]).contains("isn't allowed"));
    assert!(english(&errors[4]).contains("another program"));
    assert!(english(&errors[5]).contains("another program"));
    assert!(english(&errors[7]).contains("damaged"));
    assert!(english(&errors[11]).contains("damaged") && english(&errors[11]).contains("D:\\x"));
    assert!(english(&errors[14]).contains("isn't a workbook"));
}
