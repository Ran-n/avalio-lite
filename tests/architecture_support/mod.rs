// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 16:34:45.999177
//+ Revised: 	2026/11/23 16:34:45.999177
// -----------------------------------------------------------------------------

//! Reading `src/` for tests/architecture.rs: each file's module, whether it is UI, and every
//! `crate::…` and `super::…` path on its code lines, plus bare paths through a module the file
//! imports whole (`use crate::gradebook;` then `gradebook::ui::…`).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// One `src/` file: its path from the crate root, its module path, and its code lines.
pub struct SourceFile {
    pub path: String,
    pub module: Vec<String>,
    pub lines: Vec<(usize, String)>,
}

impl SourceFile {
    /// UI files: the composition root, the window, the entry points and every feature's `ui`.
    pub fn is_ui(&self) -> bool {
        self.is_in("app")
            || self.is_in("window")
            || self.path == "src/lib.rs"
            || self.path == "src/main.rs"
            || self.module.iter().any(|part| part == "ui")
    }

    /// Whether the file is the module `top` (`src/<top>.rs`) or one of its children.
    pub fn is_in(&self, top: &str) -> bool {
        self.module.first().is_some_and(|first| first == top)
    }

    pub fn node(&self) -> String {
        node_of(&self.module)
    }

    /// Every path this file names into the crate, with the line it is on.
    pub fn uses(&self) -> Vec<(usize, &str, Vec<String>)> {
        let aliases = self.whole_module_imports();
        let mut uses = Vec::new();
        for (number, text) in &self.lines {
            for path in paths_in(text, &aliases) {
                let resolved = match path.first().map(String::as_str) {
                    Some("super") => self.parent_of(*number).into_iter().chain(path[1..].iter().cloned()).collect(),
                    _ => path,
                };
                uses.push((*number, text.as_str(), resolved));
            }
        }
        uses
    }

    /// The top-level modules imported whole (`use crate::gradebook;`, `use crate::{lock, shortcuts};`).
    fn whole_module_imports(&self) -> BTreeSet<String> {
        let uses = self
            .lines
            .iter()
            .filter(|(_, text)| text.trim_start().trim_start_matches("pub ").starts_with("use crate::"));
        uses.flat_map(|(_, text)| paths_in(text, &BTreeSet::new()))
            .filter_map(|path| match path.as_slice() {
                [name] => Some(name.clone()),
                _ => None,
            })
            .collect()
    }

    /// The module `super::` means on this line: the file's parent, or the file itself inside its
    /// inline `mod tests { … }`, which the codebase keeps at the end of a file.
    fn parent_of(&self, line: usize) -> Vec<String> {
        let tests_start = self.lines.iter().find(|(_, text)| text.trim().ends_with("mod tests {")).map(|(n, _)| *n);
        match tests_start {
            Some(start) if line > start => self.module.clone(),
            _ => self.module[..self.module.len().saturating_sub(1)].to_vec(),
        }
    }
}

/// The dependency-graph node a module path belongs to: the top-level module, with each feature's
/// `ui` and each `platform` module a node of its own.
pub fn node_of(module: &[String]) -> String {
    match module {
        [] => "lib".to_owned(),
        [platform, name, ..] if platform == "platform" && is_identifier(name) => format!("platform::{name}"),
        [feature, ui, ..] if ui == "ui" => format!("{feature}::ui"),
        [top, ..] => top.clone(),
    }
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The crate paths on one code line, as segments after `crate::` (or with `super` first).
pub fn paths_in(text: &str, aliases: &BTreeSet<String>) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        let is_word = |i: usize| bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_';
        if !is_word(start) || (start > 0 && (is_word(start - 1) || bytes[start - 1] == b':')) {
            start += 1;
            continue;
        }
        let end = (start..bytes.len()).find(|&i| !is_word(i)).unwrap_or(bytes.len());
        let word = &text[start..end];
        if text[end..].starts_with("::")
            && (word == "crate" || word == "super" || word == "avalio_lite" || aliases.contains(word))
        {
            paths.extend(paths_after(word, &text[end + 2..]));
        }
        start = end;
    }
    paths
}

/// The crate paths that `word::` starts, `rest` being what follows the `::`: a whole group for
/// `crate::{…}`, one path otherwise (with `super` or the alias kept first).
fn paths_after(word: &str, rest: &str) -> Vec<Vec<String>> {
    let segments = path_segments(rest);
    match word {
        "crate" | "avalio_lite" if rest.starts_with('{') => group_members(rest),
        "crate" | "avalio_lite" => vec![segments],
        _ => vec![std::iter::once(word.to_owned()).chain(segments).collect()],
    }
}

fn path_segments(rest: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut rest = rest;
    loop {
        let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
        segments.push(rest[..end].to_owned());
        match rest[end..].strip_prefix("::") {
            Some(next) if next.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') => rest = next,
            _ => return segments,
        }
    }
}

/// `{a, b::c}` written on one line, as `[a]` and `[b, c]`.
fn group_members(group: &str) -> Vec<Vec<String>> {
    let close = group.find('}').expect("write one `use crate::…` per module so tests/architecture.rs can read it");
    group[1..close].split(',').map(str::trim).filter(|m| !m.is_empty()).map(path_segments).collect()
}

/// Every `.rs` file of `src/`, read for the dependency rules.
pub fn sources() -> Vec<SourceFile> {
    rust_files(&["src"]).into_iter().map(|(path, text)| source_file(path, &text)).collect()
}

/// Every `.rs` file in the crate's `roots` (folders or files), as its path from the crate root and its text.
pub fn rust_files(roots: &[&str]) -> Vec<(String, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for start in roots {
        collect(&root, &root.join(start), &mut files);
    }
    files
}

fn collect(root: &Path, path: &Path, files: &mut Vec<(String, String)>) {
    if path.is_dir() {
        for entry in fs::read_dir(path).unwrap() {
            collect(root, &entry.unwrap().path(), files);
        }
    } else if path.extension().is_some_and(|e| e == "rs") {
        let relative = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        files.push((relative, fs::read_to_string(path).unwrap()));
    }
}

/// The module `path` holds, and its code lines (comments left out).
fn source_file(path: String, text: &str) -> SourceFile {
    let mut module: Vec<String> =
        path.trim_start_matches("src/").trim_end_matches(".rs").split('/').map(str::to_owned).collect();
    if matches!(module.last().map(String::as_str), Some("mod" | "lib")) {
        module.pop();
    }
    let lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.to_owned()))
        .filter(|(_, l)| !l.trim_start().starts_with("//"))
        .collect();
    SourceFile { path, module, lines }
}
