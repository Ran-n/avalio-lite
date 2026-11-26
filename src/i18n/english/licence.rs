// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:07.942878
//+ Revised: 	2026/11/23 00:26:07.942878
// -----------------------------------------------------------------------------

//! The licence texts in English: the licence screen's notices, the sheet's warnings and the Settings
//! card.

use crate::i18n::messages::LicenceMessage;

/// A licence text in English.
pub(super) fn message(message: &LicenceMessage) -> String {
    match message {
        LicenceMessage::AskForKey => ask_for_key(),
        LicenceMessage::InvalidKey => "That key isn't valid for avalio lite. Check it and try again.".to_owned(),
        LicenceMessage::InUseElsewhere { email } => {
            format!("This key is already in use on another computer. To move it here, write to {email}.")
        }
        LicenceMessage::Revoked { email } => {
            format!("This licence has been cancelled. Write to {email} if you think this is a mistake.")
        }
        LicenceMessage::Expired { date, email } => expired(*date, email),
        LicenceMessage::Withdrawn { email } => {
            format!("avalio lite can't be licensed at the moment. Write to {email}.")
        }
        LicenceMessage::NeedsInternet => needs_internet(),
        LicenceMessage::Wait { minutes } => format!("Too many attempts. Try again in {minutes} min."),
        LicenceMessage::Unreachable => {
            "The licence server can't be reached. Check the internet connection and try again.".to_owned()
        }
        LicenceMessage::NotSaved => not_saved(),
        LicenceMessage::ConnectBy { date } => format!("Connect to the internet by {date} to keep using avalio lite."),
        LicenceMessage::EndsOn { date, email } => {
            format!("Your licence ends on {date}. Write to {email} to renew it.")
        }
        LicenceMessage::Permanent => "Permanent".to_owned(),
        LicenceMessage::Until { date } => format!("Until {date}"),
        LicenceMessage::OfflineUntil { date } => format!("Works offline until {date}"),
        LicenceMessage::Confirmed => "Licence confirmed.".to_owned(),
    }
}

/// The licence screen's first notice, with the format of a key.
fn ask_for_key() -> String {
    "avalio lite needs a licence key to run. You received it by email; it looks like this:
BRN-XXXX-XXXX-XXXX-XXXX"
        .to_owned()
}

/// The notice when a licence has ended, with its date when the server gave one.
fn expired(date: Option<&str>, email: &str) -> String {
    match date {
        Some(date) => format!("Your licence ended on {date}. Write to {email} to renew it."),
        None => format!("Your licence has ended. Write to {email} to renew it."),
    }
}

/// The notice for a computer that has to connect to the internet to check the licence.
fn needs_internet() -> String {
    "Connect to the internet so avalio lite can confirm your licence. It works offline for up to 30 days at a time."
        .to_owned()
}

/// The notice when the licence couldn't be written to this computer.
fn not_saved() -> String {
    "The licence couldn't be saved on this computer. Check that there is free space and try again; the log has the details."
        .to_owned()
}
