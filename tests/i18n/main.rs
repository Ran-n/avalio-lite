// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:18.136173
//+ Revised: 	2026/11/23 13:00:37.699964
// -----------------------------------------------------------------------------

//! Translations. Every message and error is checked in both languages, and every copy of a text is
//! compared with its golden file in `tests/golden/`. The fixtures and the tests of each topic live
//! in `tests/i18n/`.

mod app;
mod data;
mod errors;
mod language;
mod licence;
mod po;
mod sheet;
mod shortcuts;
mod spreadsheet;
mod update;

use std::fs;
use std::path::PathBuf;

use avalio_lite::error::AppError;
use avalio_lite::i18n::{Language, Message, error_message, text};

/// The details a message keeps in every language: names, paths and the value it was given.
const MESSAGE_DETAILS: [&str; 6] = ["1A", "Ana", "Exam", "D:\\x", "D:\\y", "boom"];

/// The details an error keeps in every language.
const ERROR_DETAILS: [&str; 5] = ["1A", "D:\\x", "denied", "\"x\"", "3"];

fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Every message is translated in both languages and keeps the details it was given.
fn assert_messages_translated(messages: &[Message<'static>]) {
    for message in messages {
        let (english, galician) = (text(Language::English, message), text(Language::Galician, message));
        assert_ne!(english, galician, "untranslated: {message:?}");
        for detail in MESSAGE_DETAILS {
            assert_eq!(english.contains(detail), galician.contains(detail), "{detail} lost in {galician}");
        }
    }
}

/// Every error is translated in both languages and keeps the details it was given. Galician quotes
/// with « » where English uses " ".
fn assert_errors_translated(errors: &[AppError]) {
    for error in errors {
        let (english, galician) = (error_message(Language::English, error), error_message(Language::Galician, error));
        assert_ne!(english, galician, "untranslated: {error:?}");
        for detail in ERROR_DETAILS {
            let galician_form =
                detail.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')).map(|inner| format!("«{inner}»"));
            let in_galician = galician.contains(detail) || galician_form.is_some_and(|form| galician.contains(&form));
            assert_eq!(english.contains(detail), in_galician, "{detail} lost in {galician}");
        }
    }
}

/// One of each message, from every topic.
fn every_message() -> Vec<Message<'static>> {
    [
        sheet::every_sheet_message(),
        data::every_data_message(),
        spreadsheet::every_spreadsheet_message(),
        shortcuts::every_shortcuts_message(),
        licence::every_licence_message(),
        update::every_update_message(),
        app::every_app_message(),
    ]
    .concat()
}

/// One of each error, from every group.
fn every_error() -> Vec<AppError> {
    [
        errors::every_database_error(),
        errors::every_file_error(),
        errors::every_field_error(),
        errors::every_shortcut_error(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Every message and error in `language`, one quoted line each, sorted so the order of the
/// fixtures doesn't matter. `{:?}` quoting shows a line break as `\n`.
fn rendered_texts(language: Language) -> String {
    let mut messages: Vec<String> =
        every_message().iter().map(|message| format!("{:?}", text(language, message))).collect();
    messages.sort();
    let mut errors: Vec<String> =
        every_error().iter().map(|error| format!("{:?}", error_message(language, error))).collect();
    errors.sort();
    messages.iter().chain(&errors).map(|line| format!("{line}\n")).collect()
}

/// Every message and error is checked against a copy kept in `tests/golden/`, so a refactor can't
/// change a word. `AVALIO_UPDATE_GOLDEN=1` rewrites the copies after a deliberate change to a text.
#[test]
fn every_message_and_error_matches_its_golden_copy() {
    let update = std::env::var_os("AVALIO_UPDATE_GOLDEN").is_some();
    for (language, file) in [(Language::English, "messages-en.txt"), (Language::Galician, "messages-gl.txt")] {
        let path = manifest_path(&format!("tests/golden/{file}"));
        let actual = rendered_texts(language);
        if update {
            fs::write(&path, &actual).unwrap();
        }
        let expected = fs::read_to_string(&path).unwrap_or_else(|error| panic!("missing {file}: {error}"));
        assert_eq!(actual.lines().collect::<Vec<_>>(), expected.lines().collect::<Vec<_>>(), "{file} differs");
    }
}
