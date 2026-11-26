// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:01.883291
//+ Revised: 	2026/11/25 11:43:01.359274
// -----------------------------------------------------------------------------

//! The `state` reply: what the window shows, read through the app's own models, and refused if any
//! key or value names the Windows user. `reply` is the one function the `state` command calls, and
//! the check runs inside it, so no test can get a state past it. `screen` builds the fields, `rows`
//! the sheet's pupils and their cells.

use avalio_lite::app::Session;
use avalio_lite::platform::personal::{first_naming, user_names};
use avalio_lite::ui::{AppWindow, Gradebook, Licence, PupilView, PupilViewStage, Theme, Tone, Update, UpdateBanner};
use i_slint_core::window::WindowInner;
use serde_json::{Value, json};
use slint::{ComponentHandle, Model};

use crate::common::FakeDialogs;

/// The `state` reply: the screen as it is, or the refusal if it names the Windows user.
pub fn reply(session: &Session, dialogs: &FakeDialogs) -> Result<Value, String> {
    checked(screen(session, dialogs))
}

/// The screen state, refused if any key or value names the Windows user: a capture of it could
/// become a committed baseline, and no text scanner reads pixels.
fn checked(state: Value) -> Result<Value, String> {
    match first_naming(&state, &user_names()) {
        Some(at) => Err(format!("the screen state names the Windows user: {at}")),
        None => Ok(state),
    }
}

fn screen(session: &Session, dialogs: &FakeDialogs) -> Value {
    let window = &session.window;
    let gradebook = window.global::<Gradebook>();
    let columns: Vec<String> = gradebook.get_columns().iter().map(|column| column.title.to_string()).collect();
    // 0 where a column keeps its kind's default width
    let column_widths: Vec<i32> = gradebook.get_columns().iter().map(|column| column.width).collect();
    let classes: Vec<String> = gradebook.get_classes().iter().map(|class| class.name.to_string()).collect();
    json!({
        "setup_required": window.get_setup_required(),
        "active": WindowInner::from_pub(window.window()).active(),
        "locked": window.get_locked(),
        "licensed": window.global::<Licence>().get_licensed(),
        "update": window.global::<Update>().get_banner_text().as_str(),
        "banner": banner_name(window.global::<Update>().get_banner()),
        "update_status": window.global::<Update>().get_status_line().as_str(),
        "pin_set": window.get_pin_set(),
        "language": window.get_language_code().as_str(),
        "dark": window.global::<Theme>().get_dark(),
        "error": window.get_error_message().as_str(),
        "status": window.get_status_message().as_str(),
        "classes": classes,
        "class_name": gradebook.get_class_name().as_str(),
        "columns": columns,
        "column_widths": column_widths,
        "rows": rows(window),
        "panel_index": gradebook.get_panel_index(),
        "sidebar_collapsed": gradebook.get_sidebar_collapsed(),
        "pupil_view": stage_name(window.global::<PupilView>().get_stage()),
        "pupil_shown": window.global::<PupilView>().get_pupil().full_name.as_str(),
        "database_open": session.state.borrow().connection().is_some(),
        // the links the app asked Windows to open (Settings › About), none of them really opened
        "links": dialogs.links.borrow().clone(),
    })
}

/// The sheet's pupils, in sheet order, each with its names and its cells.
fn rows(window: &AppWindow) -> Vec<Value> {
    window
        .global::<Gradebook>()
        .get_rows()
        .iter()
        .map(|row| {
            let cells: Vec<Value> = row
                .cells
                .iter()
                .map(|cell| {
                    json!({
                        "text": cell.text.as_str(),
                        "tone": tone_name(cell.tone),
                        "provisional": cell.provisional,
                        "invalid": cell.invalid,
                    })
                })
                .collect();
            json!({
                "name": row.full_name.as_str(),
                "first_name": row.first_name.as_str(),
                "surname": row.surname.as_str(),
                "cells": cells,
            })
        })
        .collect()
}

fn tone_name(tone: Tone) -> &'static str {
    match tone {
        Tone::Pass => "pass",
        Tone::Fail => "fail",
        Tone::Neutral => "neutral",
    }
}

fn stage_name(stage: PupilViewStage) -> &'static str {
    match stage {
        PupilViewStage::Menu => "menu",
        PupilViewStage::Grades => "grades",
        PupilViewStage::Pin => "pin",
        _ => "off",
    }
}

fn banner_name(banner: UpdateBanner) -> &'static str {
    match banner {
        UpdateBanner::Offer => "offer",
        UpdateBanner::Downloading => "downloading",
        _ => "hidden",
    }
}
