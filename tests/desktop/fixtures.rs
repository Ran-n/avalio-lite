// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.407610
//+ Revised: 	2026/11/25 11:43:11.954593
// -----------------------------------------------------------------------------

//! The fixtures the tests start from: the settings, database and logs of one test, the sample class
//! the screens show, and the readings of a `state` answer the tests compare. Each test builds its own
//! fixture, so no test depends on another's leftovers.

use std::path::PathBuf;

use serde_json::Value;

use crate::common::Fixture;
use crate::desktop_support::App;

pub const PIN: &str = "2468";
/// What the setup tests type as the folders, so the screen doesn't depend on where the test runs.
pub const SHOWN_FOLDER: &str = r"D:\Marks";
/// Where the mark columns start in the sample class: after the first-name and surname columns.
pub const FIRST_MARK: usize = 2;
/// The longest surname in the sample, given to Uxía so the surname column is too narrow for it.
pub const LONG_SURNAME: &str = "Fernández de la Iglesia Castelao";
/// The sample's columns once Surname moved past Exam 1.
pub const MOVED: [&str; 8] =
    ["First name", "Exam 1", "Surname", "Lab report", "Project", "Final grade", "Report card", "Notes"];
/// The sheet's "⋯" menu while every pupil is already in the class.
pub const SHEET_MENU: [&str; 8] = [
    "Show a pupil their grades…",
    "Duplicate class",
    "Duplicate with pupils",
    "Duplicate with pupils and marks",
    "Hide weights & scales",
    "Export to Excel…",
    "Import from Excel…",
    "Delete class",
];
/// The column panel's buttons, by what a screen reader says (the Move buttons' keys included).
pub const PANEL_CONTROLS: [&str; 6] =
    ["Close", "Previous column (Alt+←)", "Next column (Alt+→)", "Move left (Ctrl+←)", "Move right (Ctrl+→)", "Delete"];
pub const VK_LEFT: u16 = 0x25;

/// The settings folder of a fixture: what the app is started with.
pub fn config(fixture: &Fixture) -> PathBuf {
    fixture.path("config")
}

/// The settings point at `data\avalio-lite.db` and `logs`, relative to the scratch folder the app
/// runs in, so the settings page shows the same paths on every run.
pub fn relative_paths(fixture: &Fixture) {
    let mut config = fixture.store().load().unwrap().unwrap();
    config.database_path = PathBuf::from("data").join(avalio_lite::platform::config::DATABASE_FILE_NAME);
    config.log_dir = PathBuf::from("logs");
    fixture.store().save(&config).unwrap();
}

/// Settings with a database in `data` and logs in `logs`, and nothing else yet.
pub fn with_database() -> Fixture {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    relative_paths(&fixture);
    fixture
}

/// `with_database`, with the sample class in it.
pub fn with_sample_class() -> Fixture {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    crate::common::sample_class(state.connection().unwrap());
    state.shutdown();
    relative_paths(&fixture);
    fixture
}

/// The sample class behind a PIN: the app opens on the lock screen.
pub fn with_locked_sample() -> Fixture {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    crate::common::sample_class(state.connection().unwrap());
    state.set_pin(PIN, PIN).unwrap();
    state.shutdown();
    relative_paths(&fixture);
    fixture
}

/// `with_sample_class`, with an Info column added from the add-column menu, as a user would have.
/// A later test's captures are of a sheet that has it, so the setup repeats that click.
pub fn with_info_column() -> Fixture {
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.open_add_column_menu();
    app.click_in_menu("Info");
    app.quit();
    fixture
}

/// The sample class, and Xoán Vidal, who left the only other class there was: he is in none now.
pub fn with_a_pupil_in_no_class() -> Fixture {
    use avalio_lite::gradebook::{classes, pupils};
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        crate::common::sample_class(connection);
        let spare = classes::create_class(connection, "Spare").unwrap();
        let pupil = pupils::add_new_pupil(connection, spare, "Xoán Vidal", "").unwrap();
        pupils::remove_from_class(connection, spare, pupil).unwrap();
        classes::delete_class(connection, spare).unwrap();
    }
    state.shutdown();
    relative_paths(&fixture);
    fixture
}

/// The mark cells of a row of the sheet, the names left out.
pub fn cells(state: &Value, row: usize) -> Vec<Value> {
    state["rows"][row]["cells"].as_array().unwrap()[FIRST_MARK..].to_vec()
}

/// The sheet's column titles, the names included, in sheet order.
pub fn column_titles(state: &Value) -> Vec<String> {
    state["columns"].as_array().unwrap().iter().map(|title| title.as_str().unwrap().to_owned()).collect()
}

/// A workbook for the sample class: Ana's exam as it already is, a new pupil, and a column the
/// class doesn't have, so the report has every part.
pub fn sample_import(fixture: &Fixture) -> PathBuf {
    let path = fixture.path("marks.xlsx");
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let tab = workbook.add_worksheet();
    for (column, title) in (0u16..).zip(["Pupil", "Exam 1", "Phone"]) {
        tab.write_string(0, column, title).unwrap();
    }
    tab.write_string(1, 0, "Ana Pérez").unwrap();
    tab.write_number(1, 1, 8).unwrap();
    tab.write_string(2, 0, "Xoán Vidal").unwrap();
    tab.write_number(2, 1, 7).unwrap();
    tab.write_string(2, 2, "600 000 000").unwrap();
    workbook.save(&path).unwrap();
    path
}
