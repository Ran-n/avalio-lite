// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:07.194489
//+ Revised: 	2026/11/23 16:34:35.946683
// -----------------------------------------------------------------------------

//! The shortcuts topic of `Message`. Its texts are in `english/shortcuts.rs` and `galician/shortcuts.rs`.

/// The texts about keyboard shortcuts being changed or reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShortcutsMessage<'a> {
    ShortcutChanged { action: &'a str, keys: &'a str },
    ShortcutsReset,
}
