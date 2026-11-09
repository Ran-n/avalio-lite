// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:06.324440
//+ Revised: 	2026/11/09 10:42:02.323274
// -----------------------------------------------------------------------------

//! Helpers shared by the integration tests. Each test binary uses only some of them.
#![allow(dead_code)]

pub mod controls;
pub mod http;
pub mod licence;
pub mod logo;
pub mod win32;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

use slint::ComponentHandle;

use avalio_lite::app::{self, Ports, Session};
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::{ConfigStore, DATABASE_FILE_NAME};
use avalio_lite::platform::logging::FileLogger;
use avalio_lite::state::AppState;
use avalio_lite::update::install::Installation;
use avalio_lite::update::source::FixedRelease;
use avalio_lite::update::ui::Updates;
use avalio_lite::window::dialogs::{Dialogs, FileKind};
use avalio_lite::window::worker::InlineWorker;
use clavren_client::Client;
use licence::FakeLicence;
use log::LevelFilter;
use tempfile::TempDir;

/// A scratch folder holding the settings, database and logs of one test.
pub struct Fixture {
    pub root: TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        Self { root: tempfile::tempdir().unwrap() }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.root.path().join(name)
    }

    pub fn store(&self) -> ConfigStore {
        ConfigStore::in_dir(&self.path("config"))
    }

    /// Automatic update checks off, for tests that run the real `launch`: it would otherwise ask
    /// the real GitHub, and no test touches the network.
    pub fn without_update_checks(&self) {
        let mut config = self.store().load().unwrap().unwrap();
        config.preferences.check_updates = false;
        self.store().save(&config).unwrap();
    }

    /// Where `set_up` puts the database.
    pub fn database(&self) -> PathBuf {
        self.path("data").join(DATABASE_FILE_NAME)
    }

    pub fn state(&self) -> AppState {
        AppState::new(self.store(), leaked_logger())
    }

    /// A state with a fresh, unencrypted database in `data` and logs in `logs`.
    pub fn set_up(&self) -> AppState {
        let mut state = self.state();
        state.create_database(&self.path("data"), &self.path("logs")).unwrap();
        state
    }
}

/// Class "2º ESO A": five pupils with passing, failing and missing marks in two tests (60 % out
/// of 10, 40 % out of 5), a bonus project, a final grade, its rounding, and notes.
pub fn sample_class(connection: &rusqlite::Connection) -> i64 {
    use avalio_lite::gradebook::model::ColumnType;
    use avalio_lite::gradebook::{cells, classes, columns, pupils};
    let class = classes::create_class(connection, "2º ESO A").unwrap();
    let names = ["Ana Pérez", "Brais Otero", "Carla Iglesias", "Daniel Souto", "Uxía Fernández"];
    let marks = [["8", "4,5", "1"], ["6,5", "1", ""], ["9,5", "5", "1"], ["4", "2,5", ""], ["3", "", "0,5"]];
    let add = |kind, title: &str| {
        let id = columns::add_column(connection, class, kind, Language::English).unwrap();
        columns::rename_column(connection, id, title).unwrap();
        id
    };
    let exam = add(ColumnType::Test, "Exam 1");
    columns::set_weight(connection, exam, "60").unwrap();
    let lab = add(ColumnType::Test, "Lab report");
    columns::set_weight(connection, lab, "40").unwrap();
    columns::set_max_score(connection, lab, "5").unwrap();
    let project = add(ColumnType::Test, "Project");
    columns::set_weight(connection, project, "10").unwrap();
    columns::set_max_score(connection, project, "1").unwrap();
    columns::set_bonus(connection, project, true).unwrap();
    add(ColumnType::Weighted, "Final grade");
    add(ColumnType::Round, "Report card");
    let notes = add(ColumnType::Info, "Notes");
    for (name, row) in names.iter().zip(marks) {
        let pupil = pupils::add_new_pupil(connection, class, name).unwrap();
        for (column, mark) in [exam, lab, project].into_iter().zip(row) {
            cells::set_cell(connection, column, pupil, mark).unwrap();
        }
    }
    let daniel = pupils::list_class_pupils(connection, class).unwrap()[3].id;
    cells::set_cell(connection, notes, daniel, "Resit in June").unwrap();
    class
}

/// The Galician translation of a UI string, from the bundled `.po` file, so a test measures
/// whatever wording ships.
pub fn galician(msgid: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("lang/gl/LC_MESSAGES/avalio-lite.po");
    let source = std::fs::read_to_string(path).unwrap();
    let wanted = format!("msgid {msgid:?}");
    let mut lines = source.lines().map(str::trim_end);
    lines.by_ref().find(|line| *line == wanted).unwrap_or_else(|| panic!("no Galician entry for {msgid:?}"));
    let translated = lines.next().and_then(|line| line.strip_prefix("msgstr ")).unwrap();
    translated.trim_matches('"').to_owned()
}

/// Holds `path` open the way another program might: nobody else can open it at all.
pub fn hold_exclusively(path: &Path) -> File {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new().read(true).share_mode(0).open(path).unwrap()
}

/// Holds `path` open shared for reading and writing but not deleting, so it can still be used
/// but not removed or renamed.
pub fn hold_against_removal(path: &Path) -> File {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    OpenOptions::new().read(true).share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).open(path).unwrap()
}

/// The key the tests sign releases with, in place of the real release key.
pub fn release_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[7; 32])
}

/// A logger of its own for each state; `AppState` needs a `'static` one.
pub fn leaked_logger() -> &'static FileLogger {
    Box::leak(Box::new(FileLogger::new(LevelFilter::Info)))
}

/// Answers the app's dialogs from a script and records what was asked.
#[derive(Default)]
pub struct FakeDialogs {
    pub folders: RefCell<VecDeque<Option<PathBuf>>>,
    pub files: RefCell<VecDeque<Option<PathBuf>>>,
    /// the kind of each Open dialog, in order
    pub file_kinds: RefCell<Vec<FileKind>>,
    pub saves: RefCell<VecDeque<Option<PathBuf>>>,
    /// the kind, start folder and proposed name of each Save dialog, in order
    pub save_requests: RefCell<Vec<(FileKind, PathBuf, String)>>,
    pub answers: RefCell<VecDeque<bool>>,
    pub questions: RefCell<Vec<String>>,
    pub folder_starts: RefCell<Vec<PathBuf>>,
    pub fatal: RefCell<Vec<String>>,
    /// the pages opened in the browser, in order
    pub links: RefCell<Vec<String>>,
    /// set: opening a page fails with this reason
    pub link_failure: RefCell<Option<String>>,
}

impl FakeDialogs {
    pub fn pick_folder_next(&self, folder: Option<&Path>) {
        self.folders.borrow_mut().push_back(folder.map(Path::to_path_buf));
    }

    pub fn pick_file_next(&self, file: Option<&Path>) {
        self.files.borrow_mut().push_back(file.map(Path::to_path_buf));
    }

    pub fn save_file_next(&self, file: Option<&Path>) {
        self.saves.borrow_mut().push_back(file.map(Path::to_path_buf));
    }

    pub fn answer_next(&self, yes: bool) {
        self.answers.borrow_mut().push_back(yes);
    }
}

impl Dialogs for FakeDialogs {
    fn pick_folder(&self, start: &Path) -> Option<PathBuf> {
        self.folder_starts.borrow_mut().push(start.to_path_buf());
        self.folders.borrow_mut().pop_front().expect("the test didn't script a folder picker")
    }

    fn pick_file(&self, kind: FileKind) -> Option<PathBuf> {
        self.file_kinds.borrow_mut().push(kind);
        self.files.borrow_mut().pop_front().expect("the test didn't script a file picker")
    }

    fn save_file(&self, kind: FileKind, start: &Path, file_name: &str) -> Option<PathBuf> {
        self.save_requests.borrow_mut().push((kind, start.to_path_buf(), file_name.to_owned()));
        self.saves.borrow_mut().pop_front().expect("the test didn't script a save dialog")
    }

    fn confirm(&self, question: &str) -> bool {
        self.questions.borrow_mut().push(question.to_owned());
        self.answers.borrow_mut().pop_front().expect("the test didn't script a confirmation")
    }

    fn show_fatal(&self, message: &str) {
        self.fatal.borrow_mut().push(message.to_owned());
    }

    fn open_link(&self, url: &str) -> Result<(), String> {
        self.links.borrow_mut().push(url.to_owned());
        self.link_failure.borrow().clone().map_or(Ok(()), Err)
    }
}

/// The interface language is process-wide, so UI tests that read or change texts take turns.
static UI_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    static HEADLESS: RefCell<bool> = const { RefCell::new(false) };
}

/// The real window, wired as in the app, on Slint's headless testing backend.
pub struct Ui {
    pub fixture: Fixture,
    pub dialogs: Rc<FakeDialogs>,
    /// the computer's licence and the fake Clavren behind it; activated unless the test chose
    pub licence: FakeLicence,
    /// what "the latest release" answers; nothing until a test sets it
    pub releases: Arc<FixedRelease>,
    pub session: Session,
    _turn: MutexGuard<'static, ()>,
}

impl Ui {
    /// Opens the app on `fixture`'s settings, as a fresh start would on an English Windows.
    pub fn open(fixture: Fixture) -> Self {
        Self::open_in(fixture, Language::English)
    }

    /// Like `open`, on a Windows set to `system_language`.
    pub fn open_in(fixture: Fixture, system_language: Language) -> Self {
        let licence = FakeLicence::activated();
        let client = licence.client();
        Self::open_licensed(fixture, system_language, licence, client)
    }

    /// Opens the app on a computer that was never activated.
    pub fn open_unlicensed(fixture: Fixture) -> Self {
        let licence = FakeLicence::new();
        let client = licence.client();
        Self::open_licensed(fixture, Language::English, licence, client)
    }

    /// Opens the app with `client` for the licence of `licence`'s computer; background work runs at
    /// once, since the headless backend has no event loop.
    pub fn open_licensed(
        fixture: Fixture,
        system_language: Language,
        licence: FakeLicence,
        client: Arc<Client>,
    ) -> Self {
        Self::open_full(fixture, system_language, licence, client, None)
    }

    /// Opens the app as `installation` (a scratch exe, never the test's own), trusting updates
    /// signed with `release_key()`; downloads go to the fixture's `downloads` folder.
    pub fn open_installed(fixture: Fixture, installation: Installation) -> Self {
        let licence = FakeLicence::activated();
        let client = licence.client();
        Self::open_full(fixture, Language::English, licence, client, Some(installation))
    }

    fn open_full(
        fixture: Fixture,
        system_language: Language,
        licence: FakeLicence,
        client: Arc<Client>,
        installation: Option<Installation>,
    ) -> Self {
        let turn = UI_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        HEADLESS.with(|ready| {
            if !*ready.borrow() {
                i_slint_backend_testing::init_no_event_loop();
                *ready.borrow_mut() = true;
            }
        });
        let dialogs = Rc::new(FakeDialogs::default());
        let releases = Arc::new(FixedRelease::default());
        let updates = Updates {
            source: releases.clone(),
            key: release_key().verifying_key(),
            installation,
            downloads: fixture.path("downloads"),
        };
        let ports = Ports { dialogs: dialogs.clone(), licence: client, worker: Rc::new(InlineWorker), updates };
        let session = app::open(fixture.store(), leaked_logger(), ports, system_language).unwrap();
        // the size the app opens at (its preferred size); headless windows otherwise get a default
        session.window.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
        Self { fixture, dialogs, licence, releases, session, _turn: turn }
    }

    /// The app after first-run setup created a database in the fixture's `data` folder.
    pub fn with_database() -> Self {
        let fixture = Fixture::new();
        fixture.set_up().shutdown();
        Self::open(fixture)
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        self.session.state.borrow_mut().shutdown();
    }
}
