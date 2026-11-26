// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.562793
//+ Revised: 	2026/11/24 16:26:03.117093
// -----------------------------------------------------------------------------

//! Checking the forges for a newer release: which sources are asked, failures and the daily schedule.

use crate::common::http::{self};
use crate::common::release_key;
use crate::common::releases::{self, Published, TARGET, version};
use crate::support::{BUILD, keys, release};
use avalio_lite::update::check::{
    AFTER_FAILURE, AFTER_SUCCESS, FIRST_CHECK_DELAY, Status, ask_all, check, decide, first_check_in, next_check_in,
    offered,
};
use avalio_lite::update::download::{self, Fetch, Limits, Saving};
use avalio_lite::update::release::{CheckError, NetworkError, Release};
use avalio_lite::update::source::{
    FixedRelease, Forge, GITHUB_LATEST, GITLAB_LATEST, Http, ReleaseSource, SharedSource, Transport, USER_AGENT,
};
use avalio_lite::update::target::{
    Build, Form, asset_name, bridge_asset_name, bridge_signature_name, checksum_name, signature_name,
};
use chrono::{TimeDelta, Utc};
use proptest::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// A release with this build's file and its proof.
fn complete(text: &str) -> Release {
    let name = asset_name(&version(text), TARGET, Form::Portable);
    release(text, &[name.clone(), checksum_name(&name), signature_name(&name)])
}

fn answered(answers: Vec<(&str, Result<Release, CheckError>)>, current: &str, skipped: Option<&str>) -> Status {
    let answers = answers.into_iter().map(|(name, answer)| (name.to_owned(), answer)).collect();
    decide(answers, BUILD, &version(current), skipped.map(version).as_ref())
}

fn down() -> Result<Release, CheckError> {
    Err(CheckError::Network(NetworkError::TimedOut))
}

#[test]
fn a_newer_release_with_its_files_is_available() {
    match answered(vec![("GitHub", Ok(complete("1.1.0")))], "1.0.0", None) {
        Status::Available(offer) => {
            assert_eq!(offer.version(), &version("1.1.0"));
            assert_eq!(offer.asset.name, "avalio-lite-1.1.0-windows-x86_64-portable.exe");
            assert_eq!(offer.checksum.name, "avalio-lite-1.1.0-windows-x86_64-portable.exe.b3");
            assert_eq!(offer.signature.name, "avalio-lite-1.1.0-windows-x86_64-portable.exe.minisig");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_same_or_an_older_release_is_up_to_date() {
    assert_eq!(answered(vec![("GitHub", Ok(complete("1.0.0")))], "1.0.0", None), Status::UpToDate);
    assert_eq!(answered(vec![("GitHub", Ok(complete("0.9.0")))], "1.0.0", None), Status::UpToDate);
}

#[test]
fn a_release_missing_the_file_or_its_proof_has_no_build() {
    let name = asset_name(&version("1.1.0"), TARGET, Form::Portable);
    for missing in [name.clone(), checksum_name(&name), signature_name(&name)] {
        let names: Vec<String> =
            [name.clone(), checksum_name(&name), signature_name(&name)].into_iter().filter(|n| *n != missing).collect();
        assert_eq!(
            answered(vec![("GitHub", Ok(release("1.1.0", &names)))], "1.0.0", None),
            Status::NoBuild(version("1.1.0")),
            "without {missing}"
        );
    }
    // the bridge files alone are no build for this version, and nor is an older-format signature
    let bridge = bridge_asset_name(&version("1.1.0"), Form::Portable);
    let names = [bridge.clone(), checksum_name(&bridge), bridge_signature_name(&bridge)];
    assert_eq!(
        answered(vec![("GitHub", Ok(release("1.1.0", &names)))], "1.0.0", None),
        Status::NoBuild(version("1.1.0"))
    );
    let names = [name.clone(), checksum_name(&name), bridge_signature_name(&name)];
    assert_eq!(
        answered(vec![("GitHub", Ok(release("1.1.0", &names)))], "1.0.0", None),
        Status::NoBuild(version("1.1.0"))
    );
}

#[test]
fn a_skipped_version_stays_quiet_until_a_newer_one() {
    assert_eq!(
        answered(vec![("GitHub", Ok(complete("1.1.0")))], "1.0.0", Some("1.1.0")),
        Status::Skipped(version("1.1.0"))
    );
    assert_eq!(
        answered(vec![("GitHub", Ok(complete("1.1.0")))], "1.0.0", Some("1.2.0")),
        Status::Skipped(version("1.1.0")),
        "a newer skip covers it"
    );
    assert!(matches!(answered(vec![("GitHub", Ok(complete("1.2.0")))], "1.0.0", Some("1.1.0")), Status::Available(_)));
}

#[test]
fn a_pre_release_is_only_offered_to_a_pre_release() {
    assert!(!offered(&version("1.1.0-beta.1"), &version("1.0.0")));
    assert!(offered(&version("1.1.0-beta.2"), &version("1.1.0-beta.1")));
    assert!(offered(&version("1.1.0"), &version("1.1.0-beta.1")), "the release itself, after its betas");
    assert!(offered(&version("1.1.0"), &version("1.0.0")));
    assert!(!offered(&version("1.0.0"), &version("1.0.0")));
}

proptest! {
    #[test]
    fn what_is_offered_is_newer_and_a_pre_release_only_to_a_pre_release(
        a in (0u64..4, 0u64..4, 0u64..4, proptest::option::of(0u64..3)),
        b in (0u64..4, 0u64..4, 0u64..4, proptest::option::of(0u64..3)),
    ) {
        let make = |(major, minor, patch, pre): (u64, u64, u64, Option<u64>)| {
            let text = match pre { Some(pre) => format!("{major}.{minor}.{patch}-rc.{pre}"), None => format!("{major}.{minor}.{patch}") };
            version(&text)
        };
        let (candidate, current) = (make(a), make(b));
        let expected = candidate > current && (candidate.pre.is_empty() || !current.pre.is_empty());
        prop_assert_eq!(offered(&candidate, &current), expected);
    }
}

#[test]
fn one_source_down_either_way_still_decides() {
    assert!(matches!(
        answered(vec![("GitHub", down()), ("GitLab", Ok(complete("1.1.0")))], "1.0.0", None),
        Status::Available(_)
    ));
    assert!(matches!(
        answered(vec![("GitHub", Ok(complete("1.1.0"))), ("GitLab", down())], "1.0.0", None),
        Status::Available(_)
    ));
    assert_eq!(answered(vec![("GitHub", down()), ("GitLab", Ok(complete("1.0.0")))], "1.0.0", None), Status::UpToDate);
}

#[test]
fn the_newest_version_any_source_reports_wins() {
    let status = answered(vec![("GitHub", Ok(complete("1.1.0"))), ("GitLab", Ok(complete("1.2.0")))], "1.0.0", None);
    let Status::Available(offer) = status else { panic!("{status:?}") };
    assert_eq!(offer.version(), &version("1.2.0"), "one forge can't hide a release");
}

#[test]
fn on_a_tie_the_first_sources_files_are_used() {
    let mut gitlab = complete("1.1.0");
    for asset in &mut gitlab.assets {
        asset.url = asset.url.replace("example.org", "gitlab.example.org");
    }
    let Status::Available(offer) =
        answered(vec![("GitHub", Ok(complete("1.1.0"))), ("GitLab", Ok(gitlab))], "1.0.0", None)
    else {
        panic!()
    };
    assert!(offer.asset.url.starts_with("https://example.org/"), "{}", offer.asset.url);
}

#[test]
fn every_source_down_reports_each_reason_in_order() {
    let status = answered(
        vec![("GitHub", Err(CheckError::Network(NetworkError::Status(429)))), ("GitLab", down())],
        "1.0.0",
        None,
    );
    assert_eq!(
        status,
        Status::Failed(vec![
            ("GitHub".into(), CheckError::Network(NetworkError::Status(429))),
            ("GitLab".into(), CheckError::Network(NetworkError::TimedOut)),
        ])
    );
}

#[test]
fn a_build_from_source_asks_nobody() {
    let source = Arc::new(FixedRelease::named("GitHub"));
    source.set(Ok(complete("9.0.0")));
    let sources: Vec<SharedSource> = vec![source.clone()];
    assert_eq!(check(&sources, Build::Source, &version("1.0.0"), None), Status::SourceBuild);
    assert_eq!(source.calls(), 0);
    let answers = vec![("GitHub".to_owned(), Ok(complete("9.0.0")))];
    assert_eq!(
        decide(answers, Build::Source, &version("1.0.0"), None),
        Status::SourceBuild,
        "an answer offers it nothing either"
    );
}

/// A source that answers only once every one of `everyone` has been asked, or gives up after 10 s.
struct Together {
    name: &'static str,
    asked: Arc<AtomicUsize>,
    everyone: usize,
}

impl ReleaseSource for Together {
    fn name(&self) -> &str {
        self.name
    }
    fn latest(&self) -> Result<Release, CheckError> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.asked.load(Ordering::SeqCst) < self.everyone {
            if Instant::now() > deadline {
                return down();
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(complete("1.1.0"))
    }
}

#[test]
fn both_sources_are_asked_at_the_same_time() {
    let asked = Arc::new(AtomicUsize::new(0));
    let source = |name| -> SharedSource { Arc::new(Together { name, asked: asked.clone(), everyone: 2 }) };
    let answers = ask_all(&[source("GitHub"), source("GitLab")]);
    // one after the other, the first would have waited for the second in vain
    assert_eq!(answers, ["GitHub", "GitLab"].map(|name| (name.to_owned(), Ok(complete("1.1.0")))));
}

#[test]
fn a_source_that_breaks_counts_as_failed() {
    let broken = Arc::new(FixedRelease::named("GitHub"));
    broken.panic_on_ask();
    let fine = Arc::new(FixedRelease::named("GitLab"));
    fine.set(Ok(complete("1.1.0")));
    let sources: Vec<SharedSource> = vec![broken, fine];
    let answers = ask_all(&sources);
    assert_eq!(answers[0].1, Err(CheckError::BrokeOff));
    assert!(matches!(check(&sources, BUILD, &version("1.0.0"), None), Status::Available(_)));
}

#[test]
fn a_hanging_source_times_out() {
    let server = http::serve(HashMap::from([("/latest".to_owned(), http::hanging())]));
    let forge = Forge::github(&server.url("/latest"), Http::new(Transport::AllowHttp, Duration::from_millis(500)));
    let started = Instant::now();
    assert_eq!(forge.latest(), Err(CheckError::Network(NetworkError::TimedOut)));
    assert!(started.elapsed() < Duration::from_secs(60), "took {:?}", started.elapsed());
}

#[test]
fn both_forges_are_read_over_http() {
    let published = Published::new("1.1.0", b"new", &release_key());
    let github = Forge::github(&published.server.url("/github"), releases::http());
    let gitlab = Forge::gitlab(&published.server.url("/gitlab"), releases::http());
    assert_eq!(github.latest().unwrap(), published.release);
    assert_eq!(gitlab.latest().unwrap().assets, published.release.assets);
    assert_eq!((github.name(), gitlab.name()), ("GitHub", "GitLab"));
}

#[test]
fn a_forge_answering_another_status_or_nonsense_fails_with_the_reason() {
    let server = http::serve(HashMap::from([
        ("/429".to_owned(), http::answer("429 Too Many Requests", b"")),
        ("/nonsense".to_owned(), http::file(b"<html>")),
        ("/huge".to_owned(), http::file(&vec![b' '; 2 * 1024 * 1024])),
    ]));
    let ask = |path: &str| Forge::github(&server.url(path), releases::http()).latest();
    assert_eq!(ask("/429"), Err(CheckError::Network(NetworkError::Status(429))));
    assert_eq!(ask("/missing"), Err(CheckError::Network(NetworkError::Status(404))));
    assert!(matches!(ask("/nonsense"), Err(CheckError::Unreadable(_))));
    assert_eq!(ask("/huge"), Err(CheckError::Network(NetworkError::TooLarge)), "answers are capped at 1 MiB");
    let nowhere = Forge::github("http://127.0.0.1:9/latest", releases::http()).latest();
    assert_eq!(nowhere, Err(CheckError::Network(NetworkError::ConnectionFailed)));
}

#[test]
fn the_real_forges_are_asked_over_https_only() {
    assert!(GITHUB_LATEST.starts_with("https://api.github.com/repos/Ran-n/avalio-lite/"));
    assert!(GITLAB_LATEST.starts_with("https://gitlab.com/api/v4/projects/Ran-n%2Favalio-lite/"));
    let plain =
        Forge::github("http://example.org/latest", Http::new(Transport::HttpsOnly, Duration::from_secs(5))).latest();
    assert_eq!(plain, Err(CheckError::Network(NetworkError::NotHttps)));
}

/// A check sends the URL, Accept and User-Agent: no identifier, no personal data.
#[test]
fn requests_carry_no_identifying_data() {
    let published = Published::new("1.1.0", b"new", &release_key());
    Forge::github(&published.server.url("/github"), releases::http()).latest().unwrap();
    Forge::gitlab(&published.server.url("/gitlab"), releases::http()).latest().unwrap();
    download::download(
        &Fetch { http: &releases::http(), offer: &published.offer(), keys: &keys() },
        &Saving {
            folder: tempfile::tempdir().unwrap().path(),
            limits: Limits::default(),
            cancel: &AtomicBool::new(false),
        },
        |_| {},
    )
    .unwrap();
    let allowed = ["host", "accept", "user-agent", "accept-encoding"];
    for request in published.server.requests() {
        for header in request.lines().skip(1).filter(|line| !line.trim().is_empty()) {
            let name = header.split(':').next().unwrap().trim().to_ascii_lowercase();
            assert!(allowed.contains(&name.as_str()), "sent {header:?}");
            if name == "user-agent" {
                assert_eq!(header.split_once(':').unwrap().1.trim(), USER_AGENT);
            }
        }
    }
    assert_eq!(USER_AGENT, format!("avalio-lite/{}", env!("CARGO_PKG_VERSION")));
}

#[test]
#[ignore = "goes online to api.github.com and gitlab.com"]
fn the_real_forges_name_the_latest_release() {
    let (sources, _) = avalio_lite::update::source::configured();
    for (name, answer) in ask_all(&sources) {
        let release = answer.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(release.version >= version("0.10.0"), "{name}: {}", release.version);
    }
}

// ---------------------------------------------------------------------------------------------
// Schedule

#[test]
fn the_first_check_waits_a_minute_or_until_a_day_has_passed() {
    let now = Utc::now();
    assert_eq!(first_check_in(None, now), FIRST_CHECK_DELAY);
    assert_eq!(first_check_in(Some(now - TimeDelta::days(3)), now), FIRST_CHECK_DELAY);
    let in_an_hour = first_check_in(Some(now - TimeDelta::hours(23)), now);
    assert!(in_an_hour <= Duration::from_secs(3600) && in_an_hour > Duration::from_secs(3590), "{in_an_hour:?}");
    assert_eq!(
        first_check_in(Some(now + TimeDelta::days(30)), now),
        AFTER_SUCCESS,
        "a clock set back waits one interval at most"
    );
}

#[test]
fn after_a_check_the_next_is_a_day_or_an_hour_away() {
    assert_eq!((next_check_in(true), next_check_in(false)), (AFTER_SUCCESS, AFTER_FAILURE));
    assert_eq!((AFTER_SUCCESS, AFTER_FAILURE), (Duration::from_secs(86_400), Duration::from_secs(3_600)));
}
