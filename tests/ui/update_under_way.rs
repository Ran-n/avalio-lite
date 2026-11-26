// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 21:04:22.255451
//+ Revised: 	2026/11/24 07:24:32.926872
// -----------------------------------------------------------------------------

//! Update and restart while the download is still under way: Cancel, quit and a second click.
//! The worker holds the download's result back, as if its thread hadn't ended yet.

use crate::driver::click;
use crate::screens::{automatic_check, release, update};
use crate::update_restart::{downloads_left, offered};
use avalio_lite::ui::UpdateBanner;
use avalio_lite::update::release::{CheckError, NetworkError};
use std::fs;

#[test]
fn an_update_already_downloading_is_not_started_again() {
    let (ui, exe, published) = offered(b"the new version", |_| {});
    ui.worker.holding.set(true);
    click(&ui, "Update and restart");
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Downloading);
    update(&ui).invoke_install();
    assert_eq!(published.server.count(&published.asset_path()), 1);
    ui.worker.release(&ui.session.window);
    assert_eq!(fs::read(&exe).unwrap(), b"the new version");
}

/// Cancel after the last byte was verified, before the program is replaced.
#[test]
fn cancelling_a_verified_download_replaces_nothing_and_deletes_it() {
    let (ui, exe, _published) = offered(b"the new version", |_| {});
    ui.worker.holding.set(true);
    click(&ui, "Update and restart");
    assert_eq!(downloads_left(&ui).len(), 1, "downloaded and verified");
    update(&ui).invoke_cancel();
    ui.worker.release(&ui.session.window);
    assert_eq!(fs::read(&exe).unwrap(), b"old version");
    assert!(downloads_left(&ui).is_empty());
    assert_eq!(ui.session.updater.restart(), None);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer, "it can be tried again");
    assert!(ui.session.window.get_error_message().is_empty(), "a Cancel is no error");
}

/// The cancelled download's late result was taken for the new one's, as both have the same version:
/// the new one went on, but Cancel and quitting no longer reached it.
#[test]
fn regression_updating_again_after_cancel_keeps_the_new_download_under_way() {
    let (ui, exe, _published) = offered(b"the new version", |_| {});
    ui.worker.holding.set(true);
    click(&ui, "Update and restart");
    update(&ui).invoke_cancel();
    let late = ui.worker.take_held();
    update(&ui).invoke_install();
    for (done, payload) in late {
        done(&ui.session.window, payload);
    }
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Downloading, "the new one is still under way");
    update(&ui).invoke_cancel();
    ui.worker.release(&ui.session.window);
    assert_eq!(fs::read(&exe).unwrap(), b"old version", "and its Cancel still counts");
}

/// Whatever the daily check answers meanwhile, the download goes on, and its Cancel still counts.
#[test]
fn regression_an_automatic_check_leaves_a_download_under_way_alone() {
    let answers = [
        Ok(release("100.0.0")),
        Ok(release(env!("CARGO_PKG_VERSION"))),
        Err(CheckError::Network(NetworkError::TimedOut)),
    ];
    for answer in answers {
        let (ui, exe, published) = offered(b"the new version", |_| {});
        ui.worker.holding.set(true);
        click(&ui, "Update and restart");
        let text = update(&ui).get_banner_text();
        ui.worker.holding.set(false);
        ui.releases.set(answer.clone());
        automatic_check(&ui);
        assert_eq!(update(&ui).get_banner(), UpdateBanner::Downloading, "after {answer:?}");
        assert_eq!(update(&ui).get_banner_text(), text, "after {answer:?}");
        update(&ui).invoke_install();
        assert_eq!(published.server.count(&published.asset_path()), 1, "not started again after {answer:?}");
        update(&ui).invoke_cancel();
        ui.worker.release(&ui.session.window);
        assert_eq!(fs::read(&exe).unwrap(), b"old version", "after {answer:?}");
    }
}

#[test]
fn quitting_cancels_the_download_so_nothing_is_replaced() {
    let (ui, exe, _published) = offered(b"the new version", |_| {});
    ui.worker.holding.set(true);
    click(&ui, "Update and restart");
    ui.session.updater.quit();
    ui.worker.release(&ui.session.window);
    assert_eq!(fs::read(&exe).unwrap(), b"old version");
    assert!(downloads_left(&ui).is_empty());
}
