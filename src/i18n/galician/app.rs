// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:08.456865
//+ Revised: 	2026/11/23 00:26:08.456865
// -----------------------------------------------------------------------------

//! The texts the whole app shows when something outside a screen fails, in Galician.

use crate::i18n::messages::AppMessage;

/// An app-level text in Galician.
pub(super) fn message(message: &AppMessage) -> String {
    match message {
        AppMessage::WindowFailed => {
            "Non se puido iniciar avalio lite. Péchao e ábreo de novo; se segue pasando, o rexistro ten os detalles."
                .to_owned()
        }
        AppMessage::BackgroundFailed => "Non se puido iniciar. Téntao de novo; o rexistro ten os detalles.".to_owned(),
        AppMessage::LinkFailed { address } => format!("Non se puido abrir {address}. Cópiao no navegador."),
    }
}
