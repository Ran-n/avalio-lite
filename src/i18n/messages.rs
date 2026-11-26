// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:17.721797
//+ Revised: 	2026/11/23 00:26:14.380806
// -----------------------------------------------------------------------------

//! The texts Rust code shows, one enum per topic; `Message` joins the topics, so a translation
//! of a topic is found in its own file.

mod app;
mod data;
mod licence;
mod sheet;
mod shortcuts;
mod spreadsheet;
mod update;

use super::{Language, english, galician};
use chrono::NaiveDate;

use crate::licence::{Notice, Reminder, SUPPORT_EMAIL};

pub use app::AppMessage;
pub use data::DataMessage;
pub use licence::LicenceMessage;
pub use sheet::SheetMessage;
pub use shortcuts::ShortcutsMessage;
pub use spreadsheet::SpreadsheetMessage;
pub use update::UpdateMessage;

/// Text shown by Rust code: confirmation questions, status lines and fatal errors, by topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message<'a> {
    /// the sheet's confirmations and notes
    Sheet(SheetMessage<'a>),
    /// the database, the logs, backups, the PIN and the lock
    Data(DataMessage<'a>),
    /// Excel exports and imports
    Spreadsheet(SpreadsheetMessage<'a>),
    /// keyboard shortcuts being changed or reset
    Shortcuts(ShortcutsMessage<'a>),
    /// the licence screen, the sheet's licence warnings and the Settings card
    Licence(LicenceMessage<'a>),
    /// updates: the Settings status lines, the banner and the notes after an update
    Update(UpdateMessage<'a>),
    /// failures outside any screen
    App(AppMessage<'a>),
}

/// A day as the licence texts show it, in both languages: 30/06/2027.
pub fn licence_day(day: NaiveDate) -> String {
    day.format("%d/%m/%Y").to_string()
}

/// The sheet's warning for `reminder`.
pub fn licence_reminder(language: Language, reminder: Reminder) -> String {
    match reminder {
        Reminder::ConnectBy(day) => {
            text(language, &Message::Licence(LicenceMessage::ConnectBy { date: &licence_day(day) }))
        }
        Reminder::EndsOn(day) => {
            text(language, &Message::Licence(LicenceMessage::EndsOn { date: &licence_day(day), email: SUPPORT_EMAIL }))
        }
    }
}

/// What the licence screen says for `notice`, with the support address and dates filled in.
pub fn licence_notice(language: Language, notice: &Notice) -> String {
    let email = SUPPORT_EMAIL;
    let date = match notice {
        Notice::Expired(Some(day)) => licence_day(*day),
        _ => String::new(),
    };
    let message = match notice {
        Notice::AskForKey => LicenceMessage::AskForKey,
        Notice::InvalidKey => LicenceMessage::InvalidKey,
        Notice::InUseElsewhere => LicenceMessage::InUseElsewhere { email },
        Notice::Revoked => LicenceMessage::Revoked { email },
        Notice::Expired(day) => LicenceMessage::Expired { date: day.map(|_| date.as_str()), email },
        Notice::Withdrawn => LicenceMessage::Withdrawn { email },
        Notice::NeedsInternet => LicenceMessage::NeedsInternet,
        Notice::Wait { minutes } => LicenceMessage::Wait { minutes: *minutes },
        Notice::Unreachable => LicenceMessage::Unreachable,
        Notice::NotSaved => LicenceMessage::NotSaved,
    };
    text(language, &Message::Licence(message))
}

/// The text of `message` in `language`.
pub fn text(language: Language, message: &Message) -> String {
    match language {
        Language::English => english::message(message),
        Language::Galician => galician::message(message),
    }
}

/// `singular` for exactly one, `plural` otherwise (none included), as English and Galician count.
pub(in crate::i18n) fn plural(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}

/// "1 class", "4 classes": both languages here add the plural form for anything but one.
pub(in crate::i18n) fn counted(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// "1A → 1A (2), 1B → 1B (2)"
pub(in crate::i18n) fn renamings(pairs: &[(String, String)]) -> String {
    pairs.iter().map(|(from, to)| format!("{from} → {to}")).collect::<Vec<_>>().join(", ")
}
