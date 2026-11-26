// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.393346
//+ Revised: 	2026/11/12 14:39:53.322000
// -----------------------------------------------------------------------------

//! The licence check through Clavren, Breren's licence server: which server, product and token keys
//! this build trusts, and what the licence screen says for each answer. The client crate does the
//! activation, the offline token and the checks; the screen is wired in `ui`.

pub mod ui;

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, TimeDelta, Utc};
use clavren_client::{Client, Config, Outcome, Temporary, trusted_keys};

pub const SERVER: &str = "https://clavren.breren.com";
/// The product code, as in the Clavren dashboard.
pub const PRODUCT: &str = "avalio-lite";
/// Names the folder the client keeps its state in: `%APPDATA%\Breren\avalio-lite\licence.dat`.
pub const VENDOR: &str = "Breren";
/// Shown wherever the user has to ask for help: a key in use elsewhere, a cancelled or ended licence.
pub const SUPPORT_EMAIL: &str = "support.avalio.lite@breren.com";
/// The token-signing public keys this build trusts, from `clavren-admin show-public-key`: the key in
/// use and the next one, so the server can switch keys without locking this version out.
pub const TOKEN_KEYS: &[(&str, &str)] =
    &[("v1", "j3nwtuQyGUnpr2MY888Pjx4OM92Zk5hVjNbG1vAamug"), ("v2", "ZHQ5452QCdy6syIlT7OfwNQLyPRKw_JB6DWQRkZUe2s")];

/// The client the app uses: its state under `%APPDATA%`, protected for this Windows account, and
/// HTTPS to `SERVER`.
pub fn windows_client() -> Arc<Client> {
    // the keys are constants of this build, and a test proves they parse
    let keys = trusted_keys(TOKEN_KEYS).expect("the embedded Clavren public keys are valid");
    let config = Config::new(PRODUCT, keys, env!("CARGO_PKG_VERSION"));
    Arc::new(Client::for_windows(config, SERVER, VENDOR))
}

/// What the licence screen says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    AskForKey,
    InvalidKey,
    InUseElsewhere,
    Revoked,
    /// the licence ended, on this (local) day when known
    Expired(Option<NaiveDate>),
    /// the product was withdrawn in Clavren
    Withdrawn,
    /// 30 days offline, or the clock was set back
    NeedsInternet,
    Wait {
        minutes: u64,
    },
    Unreachable,
    /// the licence state couldn't be saved on this computer (the reason goes to the log)
    NotSaved,
}

/// The screen's text for an answer, or `None` when the app may run.
pub fn notice_for(outcome: &Outcome) -> Option<Notice> {
    Some(match outcome {
        Outcome::Licensed { .. } => return None,
        Outcome::NeedsActivation => Notice::AskForKey,
        Outcome::InvalidKey => Notice::InvalidKey,
        Outcome::AlreadyActivatedElsewhere => Notice::InUseElsewhere,
        Outcome::Revoked => Notice::Revoked,
        Outcome::Expired { at } => Notice::Expired(at.map(local_day)),
        Outcome::ProductUnavailable => Notice::Withdrawn,
        Outcome::NeedsCheck => Notice::NeedsInternet,
        Outcome::RateLimited { retry_after } => Notice::Wait { minutes: whole_minutes(*retry_after) },
        Outcome::Unreachable => Notice::Unreachable,
    })
}

/// The screen's text when a check couldn't decide anything.
pub fn notice_for_failure(failure: &Temporary) -> Notice {
    match failure {
        Temporary::Unreachable(_) | Temporary::Server(_) => Notice::Unreachable,
        Temporary::RateLimited(wait) => Notice::Wait { minutes: whole_minutes(*wait) },
        Temporary::Storage(_) => Notice::NotSaved,
    }
}

/// Whether the screen asks for a key: only while this computer has no activation. With one (an
/// ended licence, a withdrawn product, too long offline) it offers to check again instead.
pub fn asks_for_key(status: &Outcome) -> bool {
    matches!(status, Outcome::NeedsActivation | Outcome::Revoked)
}

/// The sheet warns this long before 30 days offline run out.
pub const CONNECT_WARNING: TimeDelta = TimeDelta::days(5);
/// The sheet warns this long before a licence for a set period ends: licences follow school terms
/// and are extended by hand, which takes a few days.
pub const ENDING_WARNING: TimeDelta = TimeDelta::days(14);

/// A warning on the sheet while the app still runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reminder {
    /// connect by this (local) day, or the app locks
    ConnectBy(NaiveDate),
    /// the licence ends on this (local) day
    EndsOn(NaiveDate),
}

/// The warning to show at `now`, if any. An ending licence wins over going offline, since
/// connecting can't fix it.
pub fn reminder(outcome: &Outcome, now: DateTime<Utc>) -> Option<Reminder> {
    let Outcome::Licensed { licence_expires_at, offline_until } = outcome else { return None };
    if let Some(end) = licence_expires_at
        && *end > now
        && *end - now < ENDING_WARNING
    {
        return Some(Reminder::EndsOn(local_day(*end)));
    }
    (*offline_until - now < CONNECT_WARNING).then(|| Reminder::ConnectBy(local_day(*offline_until)))
}

/// What the Settings card says about a valid licence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Standing {
    /// `None` for a permanent licence
    pub ends_on: Option<NaiveDate>,
    pub offline_until: NaiveDate,
}

pub fn standing(outcome: &Outcome) -> Option<Standing> {
    let Outcome::Licensed { licence_expires_at, offline_until } = outcome else { return None };
    Some(Standing { ends_on: licence_expires_at.map(local_day), offline_until: local_day(*offline_until) })
}

/// Minutes to wait, rounded up and never zero: "try again in 0 min" would be wrong.
fn whole_minutes(wait: Duration) -> u64 {
    wait.as_secs().div_ceil(60).max(1)
}

fn local_day(at: DateTime<Utc>) -> NaiveDate {
    at.with_timezone(&Local).date_naive()
}
