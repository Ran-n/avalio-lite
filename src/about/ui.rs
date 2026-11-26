// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 18:39:49.489085
//+ Revised: 	2026/11/23 00:26:14.700358
// -----------------------------------------------------------------------------

//! The About card's links, wired to the window: a click opens its page in the default browser (or
//! mail program).

use slint::ComponentHandle;

use super::Link;
use crate::i18n::{self, AppMessage, Message};
use crate::ui::{About, AboutLink, AppWindow};
use crate::window::{SharedDialogs, view, with_window1};

/// The link a card button names.
pub fn link(clicked: AboutLink) -> Link {
    match clicked {
        AboutLink::Website => Link::Website,
        AboutLink::Licence => Link::Licence,
        AboutLink::ThirdParty => Link::ThirdParty,
        AboutLink::Source => Link::Source,
        AboutLink::Releases => Link::Releases,
        AboutLink::Changelog => Link::Changelog,
        AboutLink::Issues => Link::Issues,
        AboutLink::Support => Link::Support,
    }
}

/// Opens the clicked link; when Windows won't, the error line gives the address to copy by hand.
pub fn wire(window: &AppWindow, dialogs: &SharedDialogs) {
    let dialogs = dialogs.clone();
    window.global::<About>().on_open(with_window1(window, move |window, clicked: AboutLink| {
        let address = link(clicked).address();
        match dialogs.open_link(&address) {
            Ok(()) => log::info!("opened {address}"),
            Err(error) => {
                log::error!("couldn't open {address}: {error}");
                let text = i18n::text(i18n::current(), &Message::App(AppMessage::LinkFailed { address: &address }));
                view::show_error_text(window, text);
            }
        }
    }));
}
