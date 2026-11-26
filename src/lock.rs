// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:41.384483
//+ Revised: 	2026/11/25 11:43:08.489073
// -----------------------------------------------------------------------------

//! PIN rules and guess limiting. The PIN itself is the database's encryption key
//! (see `db`): it is never stored, and a wrong PIN simply can't open the file.

pub mod ui;

use std::fmt;
use std::time::{Duration, Instant};

use zeroize::Zeroizing;

use crate::error::{AppError, Result};

pub const MIN_PIN_LENGTH: usize = 4;
/// Generous, so a passphrase of several words fits; only there to keep the input sane.
pub const MAX_PIN_LENGTH: usize = 128;
const FREE_ATTEMPTS: u32 = 5;
const WAIT_AFTER_FREE_ATTEMPTS: Duration = Duration::from_secs(30);

/// A PIN kept in memory while the database is open. Its bytes are overwritten with zeros when
/// it is dropped (on lock, on quit, when the PIN changes), and it never shows up in debug output.
/// This covers the app's own copy only: the text typed on the lock screen passes through Slint's
/// strings, which are freed without being zeroed.
#[derive(Clone)]
pub struct Pin(Zeroizing<String>);

impl Pin {
    pub fn new(pin: &str) -> Self {
        Self(Zeroizing::new(pin.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Pin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Pin(hidden)")
    }
}

/// A PIN is 4 to 128 characters: digits, letters, spaces, symbols, anything typeable. How long and
/// varied it is, and so how hard to guess from a copied file, is the user's call; Settings says
/// what each choice protects against.
pub fn validate_pin(pin: &str) -> Result<()> {
    let length_ok = (MIN_PIN_LENGTH..=MAX_PIN_LENGTH).contains(&pin.chars().count());
    if !length_ok || pin.chars().any(char::is_control) {
        return Err(AppError::InvalidPin);
    }
    Ok(())
}

/// Checks a new PIN and its confirmation.
pub fn validate_new_pin(pin: &str, confirmation: &str) -> Result<()> {
    validate_pin(pin)?;
    if !pins_match(pin, confirmation) {
        return Err(AppError::PinMismatch);
    }
    Ok(())
}

/// Compares two PINs in time that doesn't depend on where they first differ, so timing can't
/// reveal the PIN character by character. (Their lengths may differ; that only leaks the length.)
pub fn pins_match(first: &str, second: &str) -> bool {
    let (first, second) = (first.as_bytes(), second.as_bytes());
    first.len() == second.len() && first.iter().zip(second).fold(0, |difference, (a, b)| difference | (a ^ b)) == 0
}

/// The refusal of a try that has to wait, in whole seconds and never 0.
fn pin_wait(wait: Duration) -> AppError {
    AppError::PinWait { seconds: wait.as_secs().max(1) }
}

/// Slows down guessing at the lock screen and on the pupil view's cover, which share one count: the first few wrong PINs are free, then each
/// try has to wait. (SQLCipher's key derivation already makes every single guess slow.)
///
/// This only slows those two screens, and is kept in memory on purpose: restarting the app resets
/// it. Persisting it would add nothing, because the database file can be copied and guessed at
/// offline without going through the app at all. The real protection is the strength of the PIN
/// and SQLCipher's key derivation.
#[derive(Debug, Default)]
pub struct AttemptLimiter {
    failures: u32,
    blocked_until: Option<Instant>,
}

impl AttemptLimiter {
    /// How long until another try is allowed; `None` when one is allowed now.
    pub fn wait_remaining(&self, now: Instant) -> Option<Duration> {
        self.blocked_until.and_then(|until| until.checked_duration_since(now)).filter(|wait| !wait.is_zero())
    }

    /// Runs `check` (a PIN being tried), unless an earlier wrong PIN still has the next try waiting.
    /// A wrong PIN is counted; the one that uses up the free tries says how long the wait is.
    pub fn attempt(&mut self, now: Instant, check: impl FnOnce() -> Result<()>) -> Result<()> {
        if let Some(wait) = self.wait_remaining(now) {
            return Err(pin_wait(wait));
        }
        match check() {
            Ok(()) => {
                self.record_success();
                Ok(())
            }
            Err(AppError::WrongPin) => {
                self.record_failure(now);
                Err(self.wait_remaining(now).map_or(AppError::WrongPin, pin_wait))
            }
            Err(error) => Err(error),
        }
    }

    pub fn record_failure(&mut self, now: Instant) {
        self.failures += 1;
        if self.failures >= FREE_ATTEMPTS {
            self.blocked_until = Some(now + WAIT_AFTER_FREE_ATTEMPTS);
        }
        log::warn!("wrong PIN entered ({} in a row)", self.failures);
    }

    pub fn record_success(&mut self) {
        *self = Self::default();
    }
}
