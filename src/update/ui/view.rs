// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 03:55:27.534111
//+ Revised: 	2026/11/23 03:55:27.534111
// -----------------------------------------------------------------------------

//! What the banner and Settings › Updates show, as the updater stands, in the current language.
//! Every change of the updater ends in `show`, which redraws both: the settings lines first, then
//! the banner, one function per banner state.

use slint::{ComponentHandle, SharedString};

use crate::i18n::{self, Language, Message, UpdateMessage};
use crate::ui::{AppWindow, Update, UpdateBanner};
use crate::update::check::{Offer, Status};
use crate::update::target::{Build, current_version};

use super::updater::{Inner, Updater};

/// Shows the banner and Settings › Updates as the updater stands, in the current language: after
/// every change, and after the language changed.
pub fn show(window: &AppWindow, updater: &Updater) {
    let language = i18n::current();
    let update = window.global::<Update>();
    show_settings(&update, language, updater);
    show_banner(&update, language, updater);
}

/// Sets the lines and switches of Settings › Updates.
fn show_settings(update: &Update<'_>, language: Language, updater: &Updater) {
    let preferences = updater.state.borrow().preferences();
    let inner = updater.inner.borrow();
    let build = updater.updates.build;
    let version = current_version().to_string();
    let target = match build {
        Build::Release(target) => Some(target),
        Build::Source => None,
    };
    update.set_source_build(build == Build::Source);
    update.set_version_line(
        i18n::text(language, &Message::Update(UpdateMessage::Version { version: &version, target })).into(),
    );
    update.set_status_line(status_text(language, &inner, build).into());
    update.set_automatic(preferences.check_automatically);
    update.set_checking(inner.checking);
    update.set_skipped(preferences.skipped_version.clone().unwrap_or_default().into());
    update.set_available(inner.offer.is_some());
    update.set_downloading(inner.download.is_some());
}

/// The status line: the source build's note, the last check's result and its time, or that nothing
/// was checked yet.
fn status_text(language: Language, inner: &Inner, build: Build) -> String {
    match (&inner.status, build) {
        (_, Build::Source) => i18n::text(language, &Message::Update(UpdateMessage::SourceBuild)),
        (None, _) => i18n::text(language, &Message::Update(UpdateMessage::NotChecked)),
        (Some((status, at)), _) => status_line(language, status, &at.format("%H:%M").to_string()),
    }
}

/// The banner for what the updater has: the download under way, else the offer not dismissed,
/// else nothing.
fn show_banner(update: &Update<'_>, language: Language, updater: &Updater) {
    let inner = updater.inner.borrow();
    let offer = inner.offer.as_ref().filter(|offer| inner.dismissed.as_ref() != Some(offer.version()));
    match (&inner.download, offer) {
        (Some(running), _) => show_downloading(update, language, &running.version),
        (None, Some(offer)) => show_offer(update, language, offer),
        (None, None) => hide_banner(update),
    }
}

/// The banner while a download runs. Once it is downloading, its text stays as the progress
/// reports left it.
fn show_downloading(update: &Update<'_>, language: Language, version: &semver::Version) {
    if update.get_banner() != UpdateBanner::Downloading {
        let version = version.to_string();
        let message = Message::Update(UpdateMessage::Downloading { version: &version, progress: None });
        update.set_banner_text(i18n::text(language, &message).into());
    }
    update.set_banner(UpdateBanner::Downloading);
}

/// The banner offering the newer version.
fn show_offer(update: &Update<'_>, language: Language, offer: &Offer) {
    let version = offer.version().to_string();
    let message = Message::Update(UpdateMessage::Offer { version: &version });
    update.set_banner_text(i18n::text(language, &message).into());
    update.set_banner(UpdateBanner::Offer);
}

/// No banner: its text is cleared and it is hidden.
fn hide_banner(update: &Update<'_>) {
    update.set_banner_text(SharedString::new());
    update.set_banner(UpdateBanner::Hidden);
}

/// The text of one check's status, in the current language.
fn status_line(language: Language, status: &Status, time: &str) -> String {
    match status {
        Status::SourceBuild => i18n::text(language, &Message::Update(UpdateMessage::SourceBuild)),
        Status::UpToDate => i18n::text(language, &Message::Update(UpdateMessage::UpToDate { time })),
        Status::Available(offer) => {
            let version = offer.version().to_string();
            i18n::text(language, &Message::Update(UpdateMessage::Available { version: &version, time }))
        }
        Status::NoBuild(version) => {
            i18n::text(language, &Message::Update(UpdateMessage::NoBuild { version: &version.to_string() }))
        }
        Status::Skipped(version) => {
            i18n::text(language, &Message::Update(UpdateMessage::Skipped { version: &version.to_string(), time }))
        }
        Status::Failed(reasons) => i18n::text(language, &Message::Update(UpdateMessage::CheckFailed { reasons })),
    }
}
