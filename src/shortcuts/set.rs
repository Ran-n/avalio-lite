// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:30.882311
//+ Revised: 	2026/11/03 11:13:30.882311
// -----------------------------------------------------------------------------

//! The shortcuts in use: the defaults, the changes read from `config.toml`, and the entries that
//! had to be ignored.

use std::collections::BTreeMap;
use std::fmt;

use super::{Action, KeyCombo, ShortcutError};

/// What was wrong with a `[shortcuts]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemKind {
    UnknownAction,
    BadKeys(ShortcutError),
    /// Another action uses the same keys.
    Clash,
}

/// A `[shortcuts]` entry that was ignored, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutProblem {
    /// The entry as written: `lock = "Ctrl+"`.
    pub entry: String,
    pub kind: ProblemKind,
}

impl fmt::Display for ShortcutProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ProblemKind::UnknownAction => write!(formatter, "{}: no such action", self.entry),
            ProblemKind::BadKeys(error) => write!(formatter, "{}: {error}", self.entry),
            ProblemKind::Clash => write!(formatter, "{}: the same keys as another action", self.entry),
        }
    }
}

/// The keys of every action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcuts {
    combos: [KeyCombo; Action::ALL.len()],
}

impl Shortcuts {
    pub fn defaults() -> Self {
        Self { combos: Action::ALL.map(Action::default_combo) }
    }

    /// The defaults with the `[shortcuts]` overrides applied. Entries that name no action or don't
    /// parse are reported and ignored. Actions left on the same keys all go back to their defaults,
    /// repeatedly, until none share keys; that ends because the defaults never clash.
    pub fn from_config(overrides: &BTreeMap<String, String>) -> (Self, Vec<ShortcutProblem>) {
        let mut shortcuts = Self::defaults();
        let mut problems = Vec::new();
        let mut entries: BTreeMap<usize, String> = BTreeMap::new();
        for (name, keys) in overrides {
            let entry = format!("{name} = {keys:?}");
            let Some(action) = Action::from_config_name(name) else {
                problems.push(ShortcutProblem { entry, kind: ProblemKind::UnknownAction });
                continue;
            };
            match keys.parse::<KeyCombo>() {
                Ok(combo) => {
                    shortcuts.combos[action.index()] = combo;
                    entries.insert(action.index(), entry);
                }
                Err(error) => problems.push(ShortcutProblem { entry, kind: ProblemKind::BadKeys(error) }),
            }
        }
        loop {
            let clashing: Vec<usize> = entries
                .keys()
                .copied()
                .filter(|&index| {
                    shortcuts
                        .combos
                        .iter()
                        .enumerate()
                        .any(|(other, combo)| other != index && *combo == shortcuts.combos[index])
                })
                .collect();
            if clashing.is_empty() {
                break;
            }
            for index in clashing {
                shortcuts.combos[index] = Action::ALL[index].default_combo();
                if let Some(entry) = entries.remove(&index) {
                    problems.push(ShortcutProblem { entry, kind: ProblemKind::Clash });
                }
            }
        }
        (shortcuts, problems)
    }

    pub fn combo(&self, action: Action) -> KeyCombo {
        self.combos[action.index()]
    }

    pub fn action_for(&self, combo: &KeyCombo) -> Option<Action> {
        Action::ALL.into_iter().find(|action| self.combo(*action) == *combo)
    }

    /// These shortcuts with `action` on `combo`, or `Clash` when another action has those keys.
    pub fn with_change(&self, action: Action, combo: KeyCombo) -> Result<Shortcuts, ShortcutError> {
        if let Some(other) = self.action_for(&combo).filter(|other| *other != action) {
            return Err(ShortcutError::Clash { keys: combo, taken_by: other });
        }
        let mut changed = self.clone();
        changed.combos[action.index()] = combo;
        Ok(changed)
    }

    pub fn is_default(&self, action: Action) -> bool {
        self.combo(action) == action.default_combo()
    }

    /// The `[shortcuts]` table for `config.toml`: only the actions that differ from their defaults.
    pub fn to_config(&self) -> BTreeMap<String, String> {
        Action::ALL
            .into_iter()
            .filter(|action| !self.is_default(*action))
            .map(|action| (action.config_name().to_owned(), self.combo(action).to_string()))
            .collect()
    }
}
