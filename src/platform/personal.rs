// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/22 21:06:23.514646
//+ Revised: 	2026/11/22 21:06:23.514646
// -----------------------------------------------------------------------------

//! A guard for the desktop tests: what they capture becomes committed baselines and README
//! pictures, and no text scanner reads pixels, so nothing on screen may name the Windows user.
//! `names_the_user` is the one matcher: the test scratch folders check their own path with it, and
//! the test driver checks every screen state before it is captured. Debug builds only.

use serde_json::Value;

/// The names that would identify the person running the tests: the account name (`USERNAME`) and
/// the profile folder's name (the last part of `USERPROFILE`), without empties or repeats.
pub fn user_names() -> Vec<String> {
    let profile = std::env::var("USERPROFILE")
        .ok()
        .and_then(|path| std::path::Path::new(&path).file_name().map(|name| name.to_string_lossy().into_owned()));
    let mut names: Vec<String> = Vec::new();
    for name in [std::env::var("USERNAME").ok(), profile].into_iter().flatten() {
        let name = name.trim().to_owned();
        if !name.is_empty() && !names.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
            names.push(name);
        }
    }
    names
}

/// Whether `text` holds one of `names` as a whole word or path component, in any case: the
/// characters on either side of it are not letters or digits, so "ana" is found in
/// `C:\Users\ana\x` and "ana pérez" but never inside "banana".
pub fn names_any(text: &str, names: &[String]) -> bool {
    let text = text.to_lowercase();
    names.iter().filter(|name| !name.is_empty()).any(|name| {
        let name = name.to_lowercase();
        text.match_indices(&name).any(|(start, _)| {
            let before = text[..start].chars().next_back();
            let after = text[start + name.len()..].chars().next();
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
        })
    })
}

/// [`names_any`] for the user running this process.
pub fn names_the_user(text: &str) -> bool {
    names_any(text, &user_names())
}

/// The first key or value in `value`, at any depth, that holds one of `names`: its path, such as
/// `settings.folders[1]` or `rows[0].cells[2].text`.
pub fn first_naming(value: &Value, names: &[String]) -> Option<String> {
    walk(value, "", names)
}

fn walk(value: &Value, at: &str, names: &[String]) -> Option<String> {
    match value {
        Value::String(text) => names_any(text, names).then(|| at.to_owned()),
        Value::Array(items) => {
            items.iter().enumerate().find_map(|(index, item)| walk(item, &format!("{at}[{index}]"), names))
        }
        Value::Object(fields) => fields.iter().find_map(|(key, item)| {
            let path = if at.is_empty() { key.clone() } else { format!("{at}.{key}") };
            if names_any(key, names) { Some(path) } else { walk(item, &path, names) }
        }),
        Value::Null | Value::Bool(_) | Value::Number(_) => None,
    }
}
