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
        self.path.starts_with("src/app/")
            || self.path.starts_with("src/window/")
            || self.path == "src/lib.rs"
            || self.path == "src/main.rs"
            || self.module.iter().any(|part| part == "ui")
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
            let segments = path_segments(&text[end + 2..]);
            if word == "crate" || word == "avalio_lite" {
                if text[end + 2..].starts_with('{') {
                    paths.extend(group_members(&text[end + 2..]));
                } else {
                    paths.push(segments);
                }
            } else {
                paths.push(std::iter::once(word.to_owned()).chain(segments).collect());
            }
        }
        start = end;
    }
    paths
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

pub fn sources() -> Vec<SourceFile> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect(&root, &root.join("src"), &mut files);
    files
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<SourceFile>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(root, &path, files);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let relative = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            let mut module: Vec<String> =
                relative.trim_start_matches("src/").trim_end_matches(".rs").split('/').map(str::to_owned).collect();
            if matches!(module.last().map(String::as_str), Some("mod" | "lib")) {
                module.pop();
            }
            let text = fs::read_to_string(&path).unwrap();
            let lines = text
                .lines()
                .enumerate()
                .map(|(i, l)| (i + 1, l.to_owned()))
                .filter(|(_, l)| !l.trim_start().starts_with("//"))
                .collect();
            files.push(SourceFile { path: relative, module, lines });
        }
    }
}
