// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 12:31:54.388040
//+ Revised: 	2026/11/03 12:31:54.388040
// -----------------------------------------------------------------------------

//! The module dependency rules of decision D4 (docs/design-decisions.md): logic never depends on
//! the UI, only the entry point uses the composition root, core logic doesn't know the interface
//! language, and the modules form no cycle.

mod architecture_support;

use std::collections::{BTreeMap, BTreeSet};

use architecture_support::{SourceFile, node_of, paths_in, sources};

/// Every `path:line: text` breaking a rule; `breaks` gets each file and one of its crate paths.
fn violations(files: &[SourceFile], breaks: impl Fn(&SourceFile, &[String]) -> bool) -> Vec<String> {
    let mut found = Vec::new();
    for file in files {
        for (number, text, path) in file.uses() {
            if breaks(file, &path) {
                found.push(format!("{}:{number}: {}", file.path, text.trim()));
            }
        }
    }
    found.dedup();
    found
}

/// A cycle in the graph, written from its first node back to it, or `None`.
fn find_cycle(edges: &BTreeMap<String, BTreeSet<String>>) -> Option<Vec<String>> {
    fn visit(
        node: &str,
        edges: &BTreeMap<String, BTreeSet<String>>,
        done: &mut BTreeSet<String>,
        trail: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if let Some(at) = trail.iter().position(|n| n == node) {
            let mut cycle = trail[at..].to_vec();
            cycle.push(node.to_owned());
            return Some(cycle);
        }
        if !done.insert(node.to_owned()) {
            return None;
        }
        trail.push(node.to_owned());
        for next in edges.get(node).into_iter().flatten() {
            if let Some(cycle) = visit(next, edges, done, trail) {
                return Some(cycle);
            }
        }
        trail.pop();
        None
    }
    let mut done = BTreeSet::new();
    edges.keys().find_map(|node| visit(node, edges, &mut done, &mut Vec::new()))
}

#[test]
fn logic_does_not_depend_on_the_ui() {
    let files = sources();
    let mut found = violations(&files, |file, path| {
        !file.is_ui()
            && (matches!(path.first().map(String::as_str), Some("ui" | "app" | "window"))
                || node_of(path).ends_with("::ui"))
    });
    for file in files.iter().filter(|f| !f.is_ui()) {
        for (number, text) in &file.lines {
            if paths_in(text, &BTreeSet::from(["slint".to_owned()])).iter().any(|p| p[0] == "slint")
                || text.trim().starts_with("use slint")
            {
                found.push(format!("{}:{number}: {}", file.path, text.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "the UI framework stays at the edge and logic never depends on wiring:\n{}",
        found.join("\n")
    );
}

#[test]
fn only_main_uses_the_composition_root() {
    let found = violations(&sources(), |file, path| {
        path.first().is_some_and(|m| m == "app") && file.path != "src/main.rs" && !file.path.starts_with("src/app/")
    });
    assert!(found.is_empty(), "the composition root sits on top; nothing else uses it:\n{}", found.join("\n"));
}

#[test]
fn core_logic_does_not_use_i18n() {
    let core = |file: &SourceFile| {
        (file.path.starts_with("src/gradebook/") && !file.path.starts_with("src/gradebook/ui/"))
            || file.path.starts_with("src/platform/db/")
            || file.path == "src/platform/storage.rs"
            || file.path == "src/lock/mod.rs"
    };
    let found = violations(&sources(), |file, path| core(file) && path.first().is_some_and(|m| m == "i18n"));
    assert!(
        found.is_empty(),
        "core logic doesn't know the interface language; it takes ports such as ColumnTitles:\n{}",
        found.join("\n")
    );
}

#[test]
fn modules_have_no_cycle() {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for file in sources() {
        let from = file.node();
        for (_, _, path) in file.uses() {
            let to = node_of(&path);
            if to != from {
                edges.entry(from.clone()).or_default().insert(to);
            }
        }
    }
    if let Some(cycle) = find_cycle(&edges) {
        panic!("dependencies point one way; this cycle breaks that: {}", cycle.join(" → "));
    }
}

fn graph(pairs: &[(&str, &str)]) -> BTreeMap<String, BTreeSet<String>> {
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (from, to) in pairs {
        edges.entry((*from).to_owned()).or_default().insert((*to).to_owned());
    }
    edges
}

#[test]
fn find_cycle_reports_a_planted_cycle() {
    let cycle = find_cycle(&graph(&[("a", "b"), ("b", "c"), ("c", "a")]));
    assert_eq!(cycle, Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned(), "a".to_owned()]));
}

#[test]
fn find_cycle_accepts_a_dag() {
    assert_eq!(find_cycle(&graph(&[("a", "b"), ("a", "c"), ("b", "c")])), None);
}

#[test]
fn paths_are_read_through_groups_super_and_whole_module_imports() {
    let aliases = BTreeSet::from(["gradebook".to_owned()]);
    let read = |text: &str| paths_in(text, &aliases);
    assert_eq!(
        read("use crate::{gradebook, lock::ui};"),
        vec![vec!["gradebook".to_owned()], vec!["lock".to_owned(), "ui".to_owned()]]
    );
    assert_eq!(
        read("gradebook::ui::wire(&app);"),
        vec![vec!["gradebook".to_owned(), "ui".to_owned(), "wire".to_owned()]]
    );
    assert_eq!(read("slint::platform::x(); windows::y();"), Vec::<Vec<String>>::new());
    assert_eq!(node_of(&["platform".to_owned(), "db".to_owned(), "Database".to_owned()]), "platform::db");
}
