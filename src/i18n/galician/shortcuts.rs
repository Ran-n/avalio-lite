// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.007786
//+ Revised: 	2026/11/23 00:26:09.007786
// -----------------------------------------------------------------------------

//! The texts about keyboard shortcuts, in Galician.

use crate::i18n::messages::ShortcutsMessage;

/// A shortcut text in Galician.
pub(super) fn message(message: &ShortcutsMessage) -> String {
    match message {
        ShortcutsMessage::ShortcutChanged { action, keys } => format!("{action} agora é {keys}."),
        ShortcutsMessage::ShortcutsReset => "Todos os atallos de teclado volven ser os de orixe.".to_owned(),
    }
}
