// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:39:46.703532
//+ Revised: 	2026/11/03 11:39:46.703532
// -----------------------------------------------------------------------------

//! Safe names for a workbook's tabs and for the file itself.

/// Excel's longest tab name, in UTF-16 units.
const TAB_NAME_LIMIT: usize = 31;

const TAB_REFUSED: [char; 7] = ['[', ']', ':', '*', '?', '/', '\\'];

const FILE_REFUSED: [char; 9] = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

/// Names Windows keeps for devices: "CON.xlsx" can't be a file.
const DEVICE_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// English Excel keeps this tab name for itself.
const RESERVED_TAB: &str = "History";

/// Added to a tab name that is taken (by Excel, or by the columns tab).
pub(super) const TAKEN_SUFFIX: &str = " (1)";

const REPLACEMENT: char = '-';

/// A tab name Excel accepts: the refused characters `[ ] : * ? / \` (and an apostrophe at either
/// end) become `-`, it is cut to 31 characters, and `fallback` stands in for an empty result.
pub fn tab_name(text: &str, fallback: &str) -> String {
    let cleaned: String =
        text.chars().map(|character| if TAB_REFUSED.contains(&character) { REPLACEMENT } else { character }).collect();
    let mut name = cut_to_utf16(&cleaned, TAB_NAME_LIMIT);
    if name.is_empty() {
        return fallback.to_owned();
    }
    if name.eq_ignore_ascii_case(RESERVED_TAB) {
        name.push_str(TAKEN_SUFFIX);
    }
    for (end, at) in [(name.starts_with('\''), 0), (name.ends_with('\''), name.len() - 1)] {
        if end {
            name.replace_range(at..=at, &REPLACEMENT.to_string());
        }
    }
    name
}

/// A file name (without extension) Windows accepts: the refused characters `\ / : * ? " < > |`
/// and control characters become `-`, as do dots and spaces at the end, and a device name
/// (CON, NUL, COM1…) gets a `-` after it.
pub fn file_name(text: &str) -> String {
    let mut name: String = text
        .chars()
        .map(
            |character| {
                if FILE_REFUSED.contains(&character) || character.is_control() { REPLACEMENT } else { character }
            },
        )
        .collect();
    let kept = name.trim_end_matches(['.', ' ']).len();
    let trailing = name.len() - kept;
    name.truncate(kept);
    name.extend(std::iter::repeat_n(REPLACEMENT, trailing));
    if DEVICE_NAMES.iter().any(|device| name.eq_ignore_ascii_case(device)) {
        name.push(REPLACEMENT);
    }
    name
}

/// The longest start of `text` that is at most `limit` UTF-16 units, without splitting a character.
fn cut_to_utf16(text: &str, limit: usize) -> String {
    let mut used = 0;
    text.chars()
        .take_while(|character| {
            used += character.len_utf16();
            used <= limit
        })
        .collect()
}
