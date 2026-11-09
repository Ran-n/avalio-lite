// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:35.257330
//+ Revised: 	2026/11/08 04:36:59.226471
// -----------------------------------------------------------------------------

//! The licence logic: what the licence screen says for each answer of the Clavren client, when it
//! asks for a key, the build's own settings (server, embedded token keys), the warnings while the
//! app runs, and the daily check.

mod common;

use std::sync::mpsc;
use std::time::{Duration, Instant};

use avalio_lite::i18n::{LANGUAGES, Language, licence_notice, licence_reminder};
use avalio_lite::licence::{
    Notice, Reminder, SERVER, SUPPORT_EMAIL, Standing, TOKEN_KEYS, asks_for_key, notice_for, notice_for_failure,
    reminder, standing,
};
use chrono::{DateTime, NaiveDate, TimeDelta, TimeZone, Utc};
use clavren_client::store::StoreError;
use clavren_client::{Background, Outcome, Temporary, trusted_keys};
use common::licence::{FakeLicence, Mode};

fn licensed() -> Outcome {
    Outcome::Licensed { licence_expires_at: None, offline_until: DateTime::from_timestamp(1_800_000_000, 0).unwrap() }
}

#[test]
fn a_valid_licence_shows_no_notice() {
    assert_eq!(notice_for(&licensed()), None);
}

#[test]
fn every_refusal_has_its_notice() {
    let cases = [
        (Outcome::NeedsActivation, Notice::AskForKey),
        (Outcome::InvalidKey, Notice::InvalidKey),
        (Outcome::AlreadyActivatedElsewhere, Notice::InUseElsewhere),
        (Outcome::Revoked, Notice::Revoked),
        (Outcome::Expired { at: None }, Notice::Expired(None)),
        (Outcome::ProductUnavailable, Notice::Withdrawn),
        (Outcome::NeedsCheck, Notice::NeedsInternet),
        (Outcome::RateLimited { retry_after: Duration::from_secs(120) }, Notice::Wait { minutes: 2 }),
        (Outcome::Unreachable, Notice::Unreachable),
    ];
    for (outcome, notice) in cases {
        assert_eq!(notice_for(&outcome), Some(notice), "{outcome:?}");
    }
}

#[test]
fn an_ended_licence_names_its_local_day() {
    let at = Utc.with_ymd_and_hms(2027, 6, 30, 12, 0, 0).unwrap();
    let day = at.with_timezone(&chrono::Local).date_naive();
    assert_eq!(notice_for(&Outcome::Expired { at: Some(at) }), Some(Notice::Expired(Some(day))));
}

#[test]
fn waits_round_up_to_whole_minutes_and_never_to_zero() {
    let wait = |seconds| notice_for(&Outcome::RateLimited { retry_after: Duration::from_secs(seconds) });
    assert_eq!(wait(0), Some(Notice::Wait { minutes: 1 }));
    assert_eq!(wait(59), Some(Notice::Wait { minutes: 1 }));
    assert_eq!(wait(60), Some(Notice::Wait { minutes: 1 }));
    assert_eq!(wait(61), Some(Notice::Wait { minutes: 2 }));
}

#[test]
fn a_check_that_could_not_decide_has_its_notice() {
    assert_eq!(notice_for_failure(&Temporary::Unreachable("refused".into())), Notice::Unreachable);
    assert_eq!(notice_for_failure(&Temporary::Server("HTTP 502".into())), Notice::Unreachable);
    assert_eq!(notice_for_failure(&Temporary::RateLimited(Duration::from_secs(300))), Notice::Wait { minutes: 5 });
    let storage = Temporary::Storage(StoreError::Corrupt);
    assert_eq!(notice_for_failure(&storage), Notice::NotSaved(StoreError::Corrupt.to_string()));
}

#[test]
fn the_screen_asks_for_a_key_only_without_an_activation() {
    assert!(asks_for_key(&Outcome::NeedsActivation));
    assert!(asks_for_key(&Outcome::Revoked), "a revoked activation is cleared, so a new key can go in");
    for outcome in [Outcome::NeedsCheck, Outcome::Expired { at: None }, Outcome::ProductUnavailable, licensed()] {
        assert!(!asks_for_key(&outcome), "{outcome:?}");
    }
}

#[test]
fn the_embedded_token_keys_parse() {
    let keys = trusted_keys(TOKEN_KEYS).expect("valid keys");
    assert_eq!(keys.len(), TOKEN_KEYS.len());
    assert!(keys.contains_key("v1"));
}

#[test]
fn the_server_is_reached_over_https() {
    assert!(SERVER.starts_with("https://"), "{SERVER}");
}

#[test]
fn notices_that_need_help_name_the_support_email_in_every_language() {
    let help = [
        Notice::InUseElsewhere,
        Notice::Revoked,
        Notice::Expired(None),
        Notice::Expired(NaiveDate::from_ymd_opt(2027, 6, 30)),
        Notice::Withdrawn,
    ];
    for language in LANGUAGES {
        for notice in &help {
            assert!(licence_notice(language, notice).contains(SUPPORT_EMAIL), "{language:?} {notice:?}");
        }
    }
}

#[test]
fn an_ended_licence_shows_its_day_as_day_month_year() {
    let notice = Notice::Expired(NaiveDate::from_ymd_opt(2027, 6, 30));
    for language in LANGUAGES {
        assert!(licence_notice(language, &notice).contains("30/06/2027"), "{language:?}");
    }
    assert!(!licence_notice(Language::English, &Notice::Expired(None)).contains('/'));
}

#[test]
fn every_notice_is_worded_in_every_language() {
    let all = [
        Notice::AskForKey,
        Notice::InvalidKey,
        Notice::InUseElsewhere,
        Notice::Revoked,
        Notice::Expired(None),
        Notice::Withdrawn,
        Notice::NeedsInternet,
        Notice::Wait { minutes: 3 },
        Notice::Unreachable,
        Notice::NotSaved("disk full".into()),
    ];
    for notice in &all {
        let english = licence_notice(Language::English, notice);
        let galician = licence_notice(Language::Galician, notice);
        assert!(!english.is_empty() && !galician.is_empty(), "{notice:?}");
        assert_ne!(english, galician, "{notice:?} is translated");
    }
    assert!(licence_notice(Language::English, &Notice::Wait { minutes: 3 }).contains('3'));
    assert!(licence_notice(Language::Galician, &Notice::NotSaved("disk full".into())).contains("disk full"));
}

// ---------------------------------------------------------------------------------------------
// While the app runs: the sheet's warnings, the Settings card, the daily check

fn licensed_until(offline_days: i64, licence_days: Option<i64>, now: DateTime<Utc>) -> Outcome {
    Outcome::Licensed {
        licence_expires_at: licence_days.map(|days| now + TimeDelta::days(days)),
        offline_until: now + TimeDelta::days(offline_days),
    }
}

fn day_of(at: DateTime<Utc>) -> NaiveDate {
    at.with_timezone(&chrono::Local).date_naive()
}

#[test]
fn the_connect_warning_starts_five_days_before_the_token_runs_out() {
    let now = Utc.with_ymd_and_hms(2027, 3, 1, 9, 0, 0).unwrap();
    assert_eq!(reminder(&licensed_until(6, None, now), now), None);
    assert_eq!(reminder(&licensed_until(5, None, now), now), None);
    let close = licensed_until(4, None, now);
    assert_eq!(reminder(&close, now), Some(Reminder::ConnectBy(day_of(now + TimeDelta::days(4)))));
}

#[test]
fn the_ending_warning_starts_fourteen_days_before_the_licence_ends() {
    let now = Utc.with_ymd_and_hms(2027, 3, 1, 9, 0, 0).unwrap();
    assert_eq!(reminder(&licensed_until(30, Some(15), now), now), None);
    assert_eq!(reminder(&licensed_until(30, Some(14), now), now), None);
    let ending = licensed_until(13, Some(13), now);
    assert_eq!(reminder(&ending, now), Some(Reminder::EndsOn(day_of(now + TimeDelta::days(13)))));
}

#[test]
fn an_ending_licence_wins_over_going_offline() {
    let now = Utc.with_ymd_and_hms(2027, 3, 1, 9, 0, 0).unwrap();
    let both = licensed_until(3, Some(3), now);
    assert_eq!(reminder(&both, now), Some(Reminder::EndsOn(day_of(now + TimeDelta::days(3)))));
}

#[test]
fn a_permanent_licence_never_warns_of_its_end_and_refusals_warn_of_nothing() {
    let now = Utc.with_ymd_and_hms(2027, 3, 1, 9, 0, 0).unwrap();
    assert_eq!(reminder(&licensed_until(20, None, now), now), None);
    assert_eq!(reminder(&Outcome::Revoked, now), None);
    assert_eq!(reminder(&Outcome::NeedsCheck, now), None);
}

#[test]
fn the_card_shows_how_long_the_licence_lasts() {
    let now = Utc.with_ymd_and_hms(2027, 3, 1, 9, 0, 0).unwrap();
    let permanent = standing(&licensed_until(30, None, now)).unwrap();
    assert_eq!(permanent, Standing { ends_on: None, offline_until: day_of(now + TimeDelta::days(30)) });
    let term = standing(&licensed_until(30, Some(90), now)).unwrap();
    assert_eq!(term.ends_on, Some(day_of(now + TimeDelta::days(90))));
    assert_eq!(standing(&Outcome::NeedsActivation), None);
}

#[test]
fn the_warnings_are_worded_in_every_language() {
    let day = NaiveDate::from_ymd_opt(2027, 6, 30).unwrap();
    for language in LANGUAGES {
        assert!(licence_reminder(language, Reminder::ConnectBy(day)).contains("30/06/2027"));
        let ends = licence_reminder(language, Reminder::EndsOn(day));
        assert!(ends.contains("30/06/2027") && ends.contains(SUPPORT_EMAIL), "{ends}");
    }
    assert_ne!(
        licence_reminder(Language::English, Reminder::ConnectBy(day)),
        licence_reminder(Language::Galician, Reminder::ConnectBy(day))
    );
}

#[test]
fn a_daily_check_that_cannot_reach_the_server_reports_nothing() {
    let computer = FakeLicence::activated();
    let client = computer.client_checking_every(Duration::from_millis(10));
    let (sender, received) = mpsc::channel();
    let checks = Background::spawn(client, move |outcome| sender.send(outcome).unwrap()).unwrap();
    computer.server.set_mode(Mode::Offline);
    computer.advance(TimeDelta::hours(25));
    assert!(received.recv_timeout(Duration::from_millis(500)).is_err(), "nothing changes while the token holds");
    assert!(computer.server.calls.load(std::sync::atomic::Ordering::SeqCst) > 1, "it did try");
    checks.stop();
}

#[test]
fn the_daily_check_reports_a_revocation_once_and_stops_promptly() {
    let computer = FakeLicence::activated();
    let client = computer.client_checking_every(Duration::from_millis(10));
    let (sender, received) = mpsc::channel();
    let checks = Background::spawn(client, move |outcome| sender.send(outcome).unwrap()).unwrap();

    computer.server.set_mode(Mode::Refuse("licence_revoked", 403));
    computer.advance(TimeDelta::hours(25));
    let reported = received.recv_timeout(Duration::from_secs(5)).expect("the revocation is reported");
    assert_eq!(reported, Outcome::Revoked);
    computer.advance(TimeDelta::hours(25));
    assert!(received.recv_timeout(Duration::from_millis(300)).is_err(), "reported once");

    let started = Instant::now();
    checks.stop();
    assert!(started.elapsed() < Duration::from_secs(1), "stopping took {:?}", started.elapsed());
}

// ---------------------------------------------------------------------------------------------
// The real server (online; run with `cargo test --test licence -- --ignored`)

/// The build's server, product and keys, with nothing saved on this computer.
#[test]
#[ignore = "goes online to clavren.breren.com"]
fn the_real_server_knows_the_product_and_refuses_an_unknown_key() {
    use clavren_client::clock::SystemClock;
    use clavren_client::store::MemoryStore;
    use clavren_client::transport::HttpTransport;
    use clavren_client::{Client, Config};

    let keys = trusted_keys(TOKEN_KEYS).unwrap();
    let config = Config::new(avalio_lite::licence::PRODUCT, keys, env!("CARGO_PKG_VERSION"));
    let client = Client::new(
        config,
        Box::new(MemoryStore::default()),
        Box::new(HttpTransport::new(SERVER)),
        Box::new(SystemClock),
    );
    // well formed, so the server answers it: unknown, not a withdrawn product
    assert_eq!(client.activate("BRN-0000-0000-0000-0000").unwrap(), Outcome::InvalidKey);
}
