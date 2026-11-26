// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.375284
//+ Revised: 	2026/11/23 11:49:16.375284
// -----------------------------------------------------------------------------

//! How names are cleaned, matched and sorted: the same name in any alphabet, ignoring case and the
//! spaces at either end, with accents kept (and accents typed as marks of their own joined), and a
//! whole name written surname first split at its comma.

use crate::error::{AppError, Result};

pub fn clean_name(text: &str) -> Result<String> {
    let name = text.trim();
    if name.is_empty() {
        return Err(AppError::EmptyName);
    }
    Ok(name.to_owned())
}

/// The first name and surname as one string, how pupils are matched by name: "Ana Pérez López",
/// or just "Ana" with no surname.
pub fn full_name(first_name: &str, surname: &str) -> String {
    if surname.is_empty() { first_name.to_owned() } else { format!("{first_name} {surname}") }
}

/// A whole name written surname first, as school lists and the app's own menus write it:
/// "Pérez López, Ana" gives `("Ana", "Pérez López")`, with or without a space after the comma.
/// `None` unless there is exactly one comma with text on both sides, since anything else would be
/// a guess. The same rule as migration 008, which split the names typed before surnames existed.
pub fn split_listed_name(text: &str) -> Option<(String, String)> {
    // the characters migration 008 trims, so a name splits the same in both
    const AROUND: &[char] = &[' ', '\t', '\n', '\r', '\u{a0}'];
    let (surname, first_name) = text.split_once(',')?;
    let (surname, first_name) = (surname.trim_matches(AROUND), first_name.trim_matches(AROUND));
    if first_name.contains(',') || surname.is_empty() || first_name.is_empty() {
        return None;
    }
    Some((first_name.to_owned(), surname.to_owned()))
}

/// A surname as stored: trimmed, and unlike a first name it may be empty.
pub fn clean_surname(text: &str) -> String {
    text.trim().to_owned()
}

/// Names (and titles) are the same name ignoring case and spaces at either end, in any
/// alphabet: "ÁLVARO " is "álvaro". Accents still count: "Alvaro" is someone else.
pub fn name_match_key(name: &str) -> String {
    let mut key = String::with_capacity(name.len());
    for character in name.trim().chars().flat_map(char::to_lowercase) {
        // an accent typed as a mark of its own (a + U+0301, as some spreadsheets store it) is
        // joined to its letter, so it matches the same name typed as "á"
        match key.pop() {
            Some(base) => match composed(base, character) {
                Some(joined) => key.push(joined),
                None => key.extend([base, character]),
            },
            None => key.push(character),
        }
    }
    key
}

/// The one letter a lower-case Latin letter and a combining accent make, for the accents
/// Galician, Spanish, Portuguese and French names use.
fn composed(base: char, mark: char) -> Option<char> {
    const BASES: &str = "aeiouy";
    let row = match mark {
        '\u{301}' => "áéíóúý",
        '\u{300}' => "àèìòù ",
        '\u{302}' => "âêîôû ",
        '\u{308}' => "äëïöüÿ",
        '\u{303}' => {
            return match base {
                'a' => Some('ã'),
                'o' => Some('õ'),
                'n' => Some('ñ'),
                _ => None,
            };
        }
        '\u{327}' => return (base == 'c').then_some('ç'),
        _ => return None,
    };
    let index = BASES.chars().position(|candidate| candidate == base)?;
    row.chars().nth(index).filter(|joined| *joined != ' ')
}

/// The order names are listed in: ignoring case and accents, so "Álvaro" sorts with the A's
/// rather than after Z, with ñ as its own letter after n, as Galician and Spanish sort it.
pub fn name_sort_key(name: &str) -> String {
    const COMBINING_MARKS: std::ops::RangeInclusive<char> = '\u{300}'..='\u{36F}';
    const COMBINING_TILDE: char = '\u{303}';
    const AFTER_N: char = '\u{7F}';
    let mut key = String::with_capacity(name.len());
    for character in name.trim().chars().flat_map(char::to_lowercase) {
        let base = match character {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'ç' => 'c',
            'ñ' => {
                key.extend(['n', AFTER_N]);
                continue;
            }
            // an accent typed as a mark of its own (a + U+0301) is dropped, but n + tilde is ñ
            mark if COMBINING_MARKS.contains(&mark) => {
                if mark == COMBINING_TILDE && key.ends_with('n') {
                    key.push(AFTER_N);
                }
                continue;
            }
            other => other,
        };
        key.push(base);
    }
    key
}

#[cfg(test)]
mod tests {
    use super::{clean_surname, split_listed_name};

    #[test]
    fn a_name_written_surname_first_splits_at_its_one_comma() {
        let split = |text: &str| split_listed_name(text);
        let pair = |first: &str, surname: &str| Some((first.to_owned(), surname.to_owned()));
        assert_eq!(split("Pérez López, Ana"), pair("Ana", "Pérez López"));
        assert_eq!(split("Pérez López,Ana"), pair("Ana", "Pérez López"));
        assert_eq!(split(" de la Fuente ,\u{a0}Xoán Carlos\t"), pair("Xoán Carlos", "de la Fuente"));
        for unsplit in ["Xoán Carlos Pérez", "Pérez, Ana, B", ", Ana", "Pérez,", ",", " , "] {
            assert_eq!(split(unsplit), None, "{unsplit:?}");
        }
    }

    #[test]
    fn a_surname_is_trimmed_and_may_be_empty() {
        assert_eq!(clean_surname("  Pérez "), "Pérez");
        assert_eq!(clean_surname("   "), "");
    }
}
