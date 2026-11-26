// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 21:04:22.098075
//+ Revised: 	2026/11/23 23:19:32.226287
// -----------------------------------------------------------------------------

//! A download on its real thread, with the event loop running: its progress on the banner, Cancel,
//! Update again after Cancel, and quitting. In a binary of its own, with one test: it needs Slint's
//! testing backend with an event loop, which belongs to one thread.

mod common;

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use avalio_lite::app::{self, Ports, Session};
use avalio_lite::i18n::{self, Language, Message, UpdateMessage};
use avalio_lite::ui::{AppWindow, Update, UpdateBanner};
use avalio_lite::update::ui::automatic_check;
use avalio_lite::window::worker::{SharedWorker, ThreadWorker};
use common::event_loop::{run_for, run_until};
use common::http::{Served, file};
use common::licence::FakeLicence;
use common::releases::{Published, Running, TARGET, updates_on};
use common::{FakeDialogs, Fixture, leaked_logger, release_key};
use slint::ComponentHandle;

const NEW: [u8; 400] = [7; 400];

/// The app as a release copy at a scratch exe, offered a version whose download takes a few seconds.
struct App {
    fixture: Fixture,
    session: Session,
    published: Published,
    exe: PathBuf,
}

impl App {
    fn offered() -> Self {
        let fixture = Fixture::new();
        fixture.set_up().shutdown();
        let exe = fixture.path("avalio-lite.exe");
        fs::write(&exe, b"old version").unwrap();
        let published = Published::new("99.0.0", &NEW, &release_key());
        let slow = Served { paced: Some((1, Duration::from_millis(20))), ..file(&NEW) };
        published.serve(&published.asset_path(), slow);
        let worker: SharedWorker = Rc::new(ThreadWorker::default());
        let running = Running {
            build: avalio_lite::update::target::Build::Release(TARGET),
            exe: Some(exe.clone()),
            automatic: false,
        };
        let updates = updates_on(vec![published.source("GitHub")], running, fixture.path("downloads"), worker.clone());
        let licence = FakeLicence::activated().client();
        let ports = Ports { dialogs: Rc::new(FakeDialogs::default()), licence, worker, updates };
        let session = app::open(fixture.store(), leaked_logger(), ports, Language::English).unwrap();
        automatic_check(&session.window, &session.updater);
        let app = Self { fixture, session, published, exe };
        app.until("the offer", |window| banner(window) == UpdateBanner::Offer);
        app
    }

    fn until(&self, what: &str, seen: impl Fn(&AppWindow) -> bool + 'static) {
        let window = self.session.window.as_weak();
        run_until(what, move || seen(&window.unwrap()));
    }

    fn update(&self) -> Update<'_> {
        self.session.window.global::<Update>()
    }

    /// Update and restart, until the banner shows how far the download has come.
    fn install(&self) {
        self.update().invoke_install();
        let started = i18n::text(
            Language::English,
            &Message::Update(UpdateMessage::Downloading { version: "99.0.0", progress: None }),
        );
        self.until("the progress", move |window| {
            let shown = window.global::<Update>().get_banner_text();
            banner(window) == UpdateBanner::Downloading && shown != started && shown.ends_with(" %")
        });
    }

    fn downloads(&self) -> usize {
        fs::read_dir(self.fixture.path("downloads")).map_or(0, Iterator::count)
    }

    /// Runs the loop until no download file is left.
    fn until_no_downloads(&self) {
        let folder = self.fixture.path("downloads");
        run_until("the download file going", move || fs::read_dir(&folder).map_or(0, Iterator::count) == 0);
    }

    fn nothing_replaced(&self) {
        run_for(Duration::from_millis(200));
        assert_eq!(fs::read(&self.exe).unwrap(), b"old version");
        assert_eq!(self.session.updater.restart(), None);
        assert!(self.session.window.get_error_message().is_empty(), "{}", self.session.window.get_error_message());
    }

    fn close(self) {
        self.session.updater.quit();
        self.session.worker.join();
        self.session.state.borrow_mut().shutdown();
    }
}

fn banner(window: &AppWindow) -> UpdateBanner {
    window.global::<Update>().get_banner()
}

fn a_download_under_way_shows_its_progress_and_cancels() {
    let app = App::offered();
    app.install();
    assert_eq!(app.downloads(), 1);
    app.update().invoke_cancel();
    assert_eq!(banner(&app.session.window), UpdateBanner::Offer, "Cancel answers at once");
    app.until_no_downloads();
    app.nothing_replaced();
    app.close();
}

fn updating_again_after_cancel_downloads_afresh_and_can_be_cancelled_too() {
    let app = App::offered();
    app.install();
    app.update().invoke_cancel();
    app.update().invoke_install();
    // the first one ends meanwhile
    run_for(Duration::from_millis(500));
    assert_eq!(banner(&app.session.window), UpdateBanner::Downloading, "the second one is under way");
    assert_eq!(app.published.server.count(&app.published.asset_path()), 2);
    app.update().invoke_cancel();
    app.until_no_downloads();
    app.nothing_replaced();
    app.close();
}

fn quitting_waits_for_the_cancelled_download_to_delete_its_file() {
    let app = App::offered();
    app.install();
    app.session.updater.quit();
    assert_eq!(app.downloads(), 0, "gone by the time quit returns");
    app.nothing_replaced();
    app.close();
}

#[test]
fn a_download_on_its_own_thread_reports_cancels_and_ends_with_the_app() {
    i_slint_backend_testing::init_integration_test_with_system_time();
    a_download_under_way_shows_its_progress_and_cancels();
    updating_again_after_cancel_downloads_afresh_and_can_be_cancelled_too();
    quitting_waits_for_the_cancelled_download_to_delete_its_file();
}
