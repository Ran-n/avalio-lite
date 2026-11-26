// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:07.590541
//+ Revised: 	2026/11/23 00:26:07.590541
// -----------------------------------------------------------------------------

//! The texts the whole app shows when something outside a screen fails, in English.

use crate::i18n::messages::AppMessage;

/// An app-level text in English.
pub(super) fn message(message: &AppMessage) -> String {
    match message {
        AppMessage::WindowFailed => {
            "avalio lite couldn't start. Close it and open it again; if that keeps happening, the log has the details."
                .to_owned()
        }
        AppMessage::BackgroundFailed => "That couldn't be started. Try again; the log has the details.".to_owned(),
        AppMessage::LinkFailed { address } => format!("{address} couldn't be opened. Copy it into your browser."),
    }
}
