// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:04.307458
//+ Revised: 	2026/11/24 17:54:09.497384
// -----------------------------------------------------------------------------

//! Installing an update: a download that replaces the program, a download refused for a wrong signature
//! or checksum, a download cancelled or quit mid-way, and nothing left behind in the downloads folder.
//! The fake forge serves the releases, and the scratch exe stands in for the program.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;

use crate::common::Fixture;
use crate::desktop_support::{self, App};

use super::fixtures::{config, with_sample_class};
use super::screens::assert_on_screen;

/// The app checks, finds 9.0.0 published as `serve` says, and the banner offers it.
fn offered(fixture: &Fixture, serve: &str) -> App {
    let mut app = App::start(&config(fixture), "en");
    app.send(json!({"cmd": "release", "version": "9.0.0", "serve": serve}));
    app.click("Settings");
    app.scroll_into_view("Check for updates now");
    app.click("Check for updates now");
    app.wait_for("the check", |state| state["banner"] == "offer");
    app
}

/// The scratch exe the driver updates, and its downloads folder.
fn scratch_exe(fixture: &Fixture) -> (std::path::PathBuf, std::path::PathBuf) {
    (config(fixture).join("app").join("avalio-lite.exe"), config(fixture).join("downloads"))
}

fn left_in(folder: &Path) -> Vec<String> {
    std::fs::read_dir(folder).map_or_else(
        |_| Vec::new(),
        |entries| entries.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect(),
    )
}

/// A cancelled download's thread deletes its file as it ends, a moment after the click.
fn wait_until_empty(folder: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !left_in(folder).is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(left_in(folder).is_empty(), "left behind: {:?}", left_in(folder));
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn update_and_restart_replaces_the_program_and_closes_for_the_new_one() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = offered(&fixture, "good");
    app.click_in_banner("Update and restart");
    let goodbye = app.wait_closed(Duration::from_secs(30));
    let (exe, downloads) = scratch_exe(&fixture);
    assert_eq!(goodbye["restart"], exe.display().to_string(), "it starts the program it replaced");
    assert_eq!(std::fs::read(&exe).unwrap(), vec![b'n'; 512 * 1024], "the new version is in place");
    assert!(left_in(&downloads).is_empty(), "{:?}", left_in(&downloads));
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_wrong_signature_or_checksum_is_refused_on_screen() {
    let _turn = desktop_support::turn();
    let refusals = [
        ("bad-signature", "isn't signed with avalio lite's release key"),
        ("bad-checksum", "didn't match its checksum"),
    ];
    for (serve, said) in refusals {
        let fixture = with_sample_class();
        let mut app = offered(&fixture, serve);
        app.click_in_banner("Update and restart");
        let state = app.wait_for("the refusal", |state| !state["error"].as_str().unwrap_or_default().is_empty());
        let error = state["error"].as_str().unwrap().to_owned();
        assert!(error.contains(said), "{serve}: {state}");
        let image = app.capture();
        assert_on_screen(&image, app.located_text(&error), "the error");
        assert_eq!(state["banner"], "offer", "it can be tried again");
        let (exe, downloads) = scratch_exe(&fixture);
        assert_eq!(std::fs::read(&exe).unwrap(), b"old version", "{serve}");
        assert!(left_in(&downloads).is_empty(), "{serve}: {:?}", left_in(&downloads));
        app.quit();
    }
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn cancel_or_quit_mid_download_leaves_nothing_behind() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = offered(&fixture, "slow");
    app.click_in_banner("Update and restart");
    app.wait_for("the download", |state| state["banner"] == "downloading");
    // the percentage keeps changing while the download runs, so a settled capture never comes
    let image = app.glimpse();
    assert_on_screen(&image, app.located("Cancel the update"), "Cancel");
    app.wait_for("progress", |state| state["update"].as_str().unwrap_or_default().contains('%'));
    app.click_in_banner("Cancel the update");
    app.wait_for("the offer again", |state| state["banner"] == "offer");
    let (exe, downloads) = scratch_exe(&fixture);
    wait_until_empty(&downloads);
    app.click_in_banner("Update and restart");
    app.wait_for("the download", |state| state["banner"] == "downloading");
    let took = app.quit_timed();
    assert!(took < Duration::from_secs(10), "quitting mid-download took {took:?}");
    assert_eq!(std::fs::read(&exe).unwrap(), b"old version");
    wait_until_empty(&downloads);
}
