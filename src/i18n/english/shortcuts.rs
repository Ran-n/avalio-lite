// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:08.145873
//+ Revised: 	2026/11/23 00:26:08.145873
// -----------------------------------------------------------------------------

//! The texts about keyboard shortcuts, in English.

use crate::i18n::messages::ShortcutsMessage;

/// A shortcut text in English.
pub(super) fn message(message: &ShortcutsMessage) -> String {
    match message {
        ShortcutsMessage::ShortcutChanged { action, keys } => format!("{action} is now {keys}."),
        ShortcutsMessage::ShortcutsReset => "All keyboard shortcuts are back to their defaults.".to_owned(),
    }
}
