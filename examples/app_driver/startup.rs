// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:01.790097
//+ Revised: 	2026/11/24 17:54:03.728621
// -----------------------------------------------------------------------------

//! The driver's start: its options from the command line, and the session the window runs on (a
//! scratch settings folder, a fake licence, a fake forge and fake dialogs). The window is hidden
//! unless the run asks to see it, and it opens on the primary monitor so the scale under test is the
//! primary one's. `open` leaves the session in `DRIVEN` and hands the window back for `main` to run.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use avalio_lite::app::{self, Ports, placement};
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::ConfigStore;
use avalio_lite::platform::logging;
use avalio_lite::ui::AppWindow;
use avalio_lite::update::source::SharedSource;
use avalio_lite::update::target::Build;
use avalio_lite::window::worker::{InlineWorker, ThreadWorker};
use slint::ComponentHandle;

use crate::common;
use crate::common::FakeDialogs;
use crate::common::licence::FakeLicence;
use crate::{DRIVEN, LICENCE, releases};

const USAGE: &str = "usage: app_driver <settings folder> [en|gl] [licensed|unlicensed|offline-soon] [source]";

/// What the command line says the run is: the settings folder, the language, the licence the fake
/// Clavren starts with, and whether the build is from source (no update controls).
pub struct Options {
    config_dir: PathBuf,
    language: Language,
    licence: FakeLicence,
    build: Build,
}

impl Options {
    /// Reads the arguments after the program name, in order: folder, language, licence, build.
    /// Returns the usage line when there is no folder.
    pub fn from_args(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        let Some(config_dir) = args.next().map(PathBuf::from) else {
            return Err(USAGE.to_owned());
        };
        let language = args.next().and_then(|code| Language::from_code(&code)).unwrap_or(Language::English);
        let licence = match args.next().as_deref() {
            Some("unlicensed") => FakeLicence::new(),
            Some("offline-soon") => FakeLicence::at(chrono::Utc::now() - chrono::TimeDelta::days(26)).activate(),
            _ => FakeLicence::activated(),
        };
        let build = match args.next().as_deref() {
            Some("source") => Build::Source,
            _ => Build::Release(common::releases::TARGET),
        };
        Ok(Self { config_dir, language, licence, build })
    }
}

/// Builds the session and opens the window on it. The error is the line `main` prints before it exits.
pub fn open(options: Options) -> Result<AppWindow, String> {
    let Options { config_dir, language, licence, build } = options;
    let logger = logging::install();
    // the window stays off the user's screen unless the run asks to see it
    if !common::win32::window_shown()
        && let Err(error) = common::win32::hide_windows_of_this_thread()
    {
        return Err(format!("the window couldn't be hidden: {error}"));
    }
    let dialogs = Rc::new(FakeDialogs::default());
    // the fake answers at once, so its work runs inline and every reply sees its result
    let (client, worker) = (licence.client(), Rc::new(InlineWorker));
    LICENCE.with(|cell| *cell.borrow_mut() = Some(licence));
    // never the driver's own exe: Update and restart replaces a scratch one in the settings folder
    let scratch = config_dir.join("app").join("avalio-lite.exe");
    std::fs::create_dir_all(config_dir.join("app"))
        .and_then(|()| std::fs::write(&scratch, b"old version"))
        .map_err(|error| format!("the scratch exe couldn't be made: {error}"))?;
    let sources: Vec<SharedSource> = vec![Arc::new(releases::DriverSource)];
    // the updater's own threads, so a download runs while the window answers clicks
    let updates = common::releases::updates_on(
        sources,
        common::releases::Running { build, exe: Some(scratch), automatic: false },
        config_dir.join("downloads"),
        Rc::new(ThreadWorker::default()),
    );
    let ports = Ports { dialogs: dialogs.clone(), licence: client, worker, updates };
    let session = app::open(ConfigStore::in_dir(&config_dir), logger, ports, language)
        .map_err(|reason| format!("the window failed: {reason}"))?;
    let window = session.window.clone_strong();
    // the app opens on the monitor under the mouse; the tests pin it to the primary one, so the display
    // scale under test is the one set there, wherever the user's mouse is
    placement::apply_with(&window, placement::work_area_at(placement::PRIMARY_SCREEN));
    DRIVEN.with(|driven| *driven.borrow_mut() = Some((session, dialogs)));
    Ok(window)
}
