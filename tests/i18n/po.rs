// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.638153
//+ Revised: 	2026/11/23 00:26:09.638153
// -----------------------------------------------------------------------------

//! The Slint strings: every `@tr` in the UI has a translation in each `.po` file, with no stale
//! entries and with its placeholders kept.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use avalio_lite::i18n::Language;

use super::manifest_path;

/// English language is the source text in the .slint files; every other language needs a .po file.
const TRANSLATED: [Language; 1] = [Language::Galician];

/// Every string passed to @tr(...) in the UI, with plural forms joined as "singular|plural".
fn ui_strings() -> BTreeSet<String> {
    ui_strings_in(&manifest_path("ui"))
}

/// The @tr strings of every .slint file under `root`, subfolders included.
fn ui_strings_in(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(ui_strings_in(&path));
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "slint") {
            continue;
        }
        let source = fs::read_to_string(&path).unwrap();
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

/// The most times `placeholder` appears on one of the `.po` entry's lines that start with `prefix`;
/// `None` when no line does.
fn placeholder_count(block: &str, prefix: &str, placeholder: &str) -> Option<usize> {
    block.lines().filter(|line| line.starts_with(prefix)).map(|line| line.matches(placeholder).count()).max()
}

#[test]
fn placeholders_survive_translation() {
    for language in TRANSLATED {
        let source = read_po(language);
        source.split("\n\n").for_each(check_placeholders);
    }
}

/// Asserts every placeholder in the `.po` entry's source text appears as often in its translation.
fn check_placeholders(block: &str) {
    for placeholder in ["{}", "{n}"] {
        let in_source = placeholder_count(block, "msgid ", placeholder);
        if in_source.is_some() {
            let translated = placeholder_count(block, "msgstr", placeholder);
            assert_eq!(translated, in_source, "{placeholder} lost in:\n{block}");
        }
    }
}

#[test]
fn ui_strings_include_nested_folders() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.slint"), r#"Text { text: @tr("Top"); }"#).unwrap();
    fs::create_dir(root.path().join("sub")).unwrap();
    fs::write(root.path().join("sub").join("b.slint"), r#"Text { text: @tr("Nested"); }"#).unwrap();
    fs::write(root.path().join("sub").join("notes.txt"), r#"@tr("Ignored")"#).unwrap();
    assert_eq!(ui_strings_in(root.path()), BTreeSet::from(["Top".to_owned(), "Nested".to_owned()]));
}
