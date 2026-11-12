// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.463651
//+ Revised: 	2026/11/12 14:39:53.406193
// -----------------------------------------------------------------------------

//! The licence screen's wiring: what it shows for each answer of the licence client, activating a
//! key or checking again on the worker, so the window never waits for the network, the Settings
//! card, the sheet's warnings, and the daily check while the app runs.

use std::sync::Arc;

use chrono::Utc;
use clavren_client::{Background, Client, Outcome};
use slint::{ComponentHandle, SharedString};

use crate::i18n::{self, Message};
use crate::licence::{Notice, asks_for_key, notice_for, notice_for_failure, reminder, standing};
use crate::ui::{AppWindow, Licence};
use crate::window::worker::{self, Panicked, SharedWorker};
use crate::window::{view, with_window, with_window1};

/// What an activation or check came back with: the client's answer, or why there was none.
pub type Answer = Result<Outcome, Notice>;

pub fn wire(window: &AppWindow, client: &Arc<Client>, worker: &SharedWorker) {
    let licence = window.global::<Licence>();

    let (shared, work) = (client.clone(), worker.clone());
    licence.on_activate(with_window1(window, move |window, key: SharedString| {
        let (client, key) = (shared.clone(), key.to_string());
        let job = move || {
            client.activate(&key).map_err(|error| {
                log::error!("the licence couldn't be saved: {error}");
                Notice::NotSaved
            })
        };
        let client = shared.clone();
        start(window, &work, job, move |window, answer| show(window, &client, &answer));
    }));

    let (shared, work) = (client.clone(), worker.clone());
    licence.on_check_again(with_window(window, move |window| {
        let client = shared.clone();
        let job = move || check(&client);
        let client = shared.clone();
        start(window, &work, job, move |window, answer| show(window, &client, &answer));
    }));

    let (shared, work) = (client.clone(), worker.clone());
    licence.on_check_now(with_window(window, move |window| {
        let client = shared.clone();
        let job = move || check(&client);
        let client = shared.clone();
        start(window, &work, job, move |window, answer| checked_now(window, &client, &answer));
    }));
}

/// Shows the app for a valid licence, or the licence screen saying what is wrong.
pub fn show(window: &AppWindow, client: &Client, answer: &Answer) {
    let licence = window.global::<Licence>();
    licence.set_busy(false);
    let notice = match answer {
        Ok(outcome) => notice_for(outcome),
        Err(notice) => Some(notice.clone()),
    };
    match (answer, notice) {
        (Ok(outcome), None) => {
            licence.set_key(SharedString::new());
            licence.set_notice(SharedString::new());
            // a backup made from the licence screen said so there; the sheet starts clean
            if !licence.get_licensed() {
                view::clear_messages(window);
            }
            licence.set_licensed(true);
            show_standing(window, outcome);
        }
        (_, notice) => {
            let notice = notice.unwrap_or(Notice::Unreachable);
            log::info!("licence screen: {notice:?}");
            licence.set_notice(i18n::licence_notice(i18n::current(), &notice).into());
            licence.set_asks_for_key(asks_for_key(&client.status()));
            licence.set_licensed(false);
        }
    }
}

/// The daily check while the app runs: whenever the last successful one is a day old, on the
/// client's own thread. A change of answer reaches the window through the event loop.
pub fn start_checks(window: &AppWindow, client: &Arc<Client>) -> std::io::Result<Background> {
    let weak = window.as_weak();
    let shared = client.clone();
    Background::spawn(client.clone(), move |outcome| {
        let client = shared.clone();
        let handed = weak.upgrade_in_event_loop(move |window| show(&window, &client, &Ok(outcome)));
        if let Err(error) = handed {
            log::info!("a licence check arrived after the window closed: {error}");
        }
    })
}

/// Settings → Licence → Check now: a definitive answer applies as anywhere else; one that can't
/// decide (no internet, say) is only reported, and the app keeps running on its token.
fn checked_now(window: &AppWindow, client: &Client, answer: &Answer) {
    match answer {
        Ok(_) => {
            show(window, client, answer);
            if window.global::<Licence>().get_licensed() {
                view::show_status(window, i18n::text(i18n::current(), &Message::LicenceConfirmed));
            }
        }
        Err(notice) => {
            window.global::<Licence>().set_busy(false);
            view::show_error_text(window, i18n::licence_notice(i18n::current(), notice));
        }
    }
}

/// The Settings card's lines and the sheet's warning for a valid licence.
fn show_standing(window: &AppWindow, outcome: &Outcome) {
    let licence = window.global::<Licence>();
    let language = i18n::current();
    if let Some(standing) = standing(outcome) {
        let term = match standing.ends_on {
            Some(day) => i18n::text(language, &Message::LicenceUntil { date: &i18n::licence_day(day) }),
            None => i18n::text(language, &Message::LicencePermanent),
        };
        licence.set_term(term.into());
        let offline = i18n::licence_day(standing.offline_until);
        licence.set_offline_until(i18n::text(language, &Message::LicenceOfflineUntil { date: &offline }).into());
    }
    if let Some(reminder) = reminder(outcome, Utc::now()) {
        view::show_status(window, i18n::licence_reminder(language, reminder));
    }
}

fn check(client: &Client) -> Answer {
    client.check().map_err(|failure| {
        log::warn!("the licence check couldn't decide: {failure}");
        notice_for_failure(&failure)
    })
}

/// Marks the screen busy and runs `job` on the worker; `done` gets its answer.
fn start(
    window: &AppWindow,
    worker: &SharedWorker,
    job: impl FnOnce() -> Answer + Send + 'static,
    done: impl FnOnce(&AppWindow, Answer) + 'static,
) {
    let licence = window.global::<Licence>();
    licence.set_busy(true);
    view::clear_messages(window);
    let finished = move |window: &AppWindow, answer: Result<Answer, Panicked>| match answer {
        Ok(answer) => done(window, answer),
        Err(Panicked) => failed(window),
    };
    if let Err(error) = worker::run(worker.as_ref(), window, job, finished) {
        log::error!("the licence check couldn't be started: {error}");
        failed(window);
    }
}

/// The check couldn't run (it couldn't start, or it broke off): the screen stops waiting and says so.
fn failed(window: &AppWindow) {
    window.global::<Licence>().set_busy(false);
    view::show_error_text(window, i18n::text(i18n::current(), &Message::BackgroundFailed));
}
