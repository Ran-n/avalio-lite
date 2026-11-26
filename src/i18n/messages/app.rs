// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:06.763567
//+ Revised: 	2026/11/23 16:34:35.683462
// -----------------------------------------------------------------------------

//! The app topic of `Message`. Its texts are in `english/app.rs` and `galician/app.rs`.

/// The texts the whole app shows when something outside a screen fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMessage<'a> {
    /// the window couldn't be created or run; the reason is in the log, not in the dialog
    WindowFailed,
    /// work meant to run in the background (a check or a download) couldn't be started
    BackgroundFailed,
    /// Windows wouldn't open a link (Settings › About); the address, to copy by hand
    LinkFailed { address: &'a str },
}
