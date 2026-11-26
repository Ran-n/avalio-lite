// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:06.976695
//+ Revised: 	2026/11/23 16:34:35.814258
// -----------------------------------------------------------------------------

//! The licence topic of `Message`. Its texts are in `english/licence.rs` and `galician/licence.rs`.

/// The licence texts: the licence screen's notices, the sheet's warnings and the Settings card.
/// The variants are named without their `Licence` prefix, which the enum's name already gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenceMessage<'a> {
    AskForKey,
    InvalidKey,
    InUseElsewhere {
        email: &'a str,
    },
    Revoked {
        email: &'a str,
    },
    /// `date` is already formatted (day/month/year); `None` when the server didn't say
    Expired {
        date: Option<&'a str>,
        email: &'a str,
    },
    Withdrawn {
        email: &'a str,
    },
    NeedsInternet,
    Wait {
        minutes: u64,
    },
    Unreachable,
    /// the licence couldn't be written to this computer (the details go to the log)
    NotSaved,
    /// on the sheet: 30 days offline run out on `date` (formatted)
    ConnectBy {
        date: &'a str,
    },
    /// on the sheet: a licence for a set period ends on `date` (formatted)
    EndsOn {
        date: &'a str,
        email: &'a str,
    },
    /// the Settings card: how long the licence lasts
    Permanent,
    Until {
        date: &'a str,
    },
    OfflineUntil {
        date: &'a str,
    },
    /// Check now found the licence valid
    Confirmed,
}
