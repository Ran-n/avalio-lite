// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.837252
//+ Revised: 	2026/11/23 13:00:32.837252
// -----------------------------------------------------------------------------

//! Helpers for particular screens that several feature modules share: the backup and licence
//! pages, the menus, the column widths and the update screens.

use crate::common::licence::FakeLicence;
use crate::common::{Fixture, Ui};
use crate::driver::{click, elements, find, gradebook, key, shows_text};
use crate::fixtures::{open_settings, scroll_settings_to, scroll_to_the_folder_cards};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{classes, columns, pupils};
use avalio_lite::i18n::{self, Language};
use avalio_lite::licence::Notice;
use avalio_lite::ui::{ColumnType as UiColumnType, Licence, Update};
use avalio_lite::update::release::{Asset, Release};
use avalio_lite::update::target::{checksum_name, signature_name};
use i_slint_backend_testing::ElementHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};
use std::fs;
use std::sync::atomic::Ordering;

// ---------------------------------------------------------------------------------------------
// Settings: backup

/// The settings page, scrolled down to the Backup card.
pub(crate) fn open_backup_settings(ui: &Ui) {
    open_settings(ui);
    scroll_to_the_folder_cards(ui);
}

/// A backup of another database with classes 7A (two pupils) and 7B, encrypted with `pin` if given.
pub(crate) fn backup_file(pin: Option<&str>) -> (Fixture, std::path::PathBuf) {
    let other = Fixture::new();
    let mut state = other.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "7A").unwrap();
        pupils::add_new_pupil(connection, class, "Xoán", "").unwrap();
        pupils::add_new_pupil(connection, class, "Iria", "").unwrap();
        classes::create_class(connection, "7B").unwrap();
    }
    if let Some(pin) = pin {
        state.set_pin(pin, pin).unwrap();
    }
    let file = other.path("avalio-lite-backup-2026-10-28.avalio");
    state.export_backup(&file).unwrap();
    state.shutdown();
    (other, file)
}

// ---------------------------------------------------------------------------------------------
// Excel

pub(crate) fn export_through_the_menu(ui: &Ui, destination: Option<&std::path::Path>) {
    click(ui, "More actions");
    ui.dialogs.save_file_next(destination);
    click(ui, "Export to Excel…");
}

/// A workbook in the fixture folder whose first tab holds `rows`, every cell written as text
/// except numbers, which are written as numbers.
pub(crate) fn import_file(ui: &Ui, rows: &[&[&str]]) -> std::path::PathBuf {
    let path = ui.fixture.path("import.xlsx");
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let tab = workbook.add_worksheet();
    for (row, cells) in (0u32..).zip(rows) {
        for (column, cell) in (0u16..).zip(*cells) {
            match cell.parse::<f64>() {
                Ok(number) => tab.write_number(row, column, number).unwrap(),
                Err(_) if cell.starts_with('#') => {
                    tab.write_formula(row, column, rust_xlsxwriter::Formula::new("=1/0").set_result(*cell)).unwrap()
                }
                Err(_) => tab.write_string(row, column, *cell).unwrap(),
            };
        }
    }
    workbook.save(&path).unwrap();
    path
}

pub(crate) fn import_through_the_menu(ui: &Ui, file: Option<&std::path::Path>) {
    click(ui, "More actions");
    ui.dialogs.pick_file_next(file);
    click(ui, "Import from Excel…");
}

/// Class "1A" with Ana and Bea and four columns: tests "Exam" and "Quiz", a final grade, and a
/// rounding of the final grade.
pub(crate) fn four_columns() -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, class, "Ana", "").unwrap();
        pupils::add_new_pupil(connection, class, "Bea", "").unwrap();
        for (column_type, title) in
            [(ColumnType::Test, "Exam"), (ColumnType::Test, "Quiz"), (ColumnType::Weighted, "Final grade")]
        {
            let id = columns::add_column(connection, class, column_type, Language::English).unwrap();
            columns::rename_column(connection, id, title).unwrap();
        }
        columns::add_column(connection, class, ColumnType::Round, Language::English).unwrap();
    }
    state.shutdown();
    Ui::open(fixture)
}

/// The titles of the mark columns in sheet order, without the first-name and surname columns.
pub(crate) fn column_titles(ui: &Ui) -> Vec<String> {
    gradebook(ui)
        .get_columns()
        .iter()
        .filter(|column| !matches!(column.kind, UiColumnType::FirstName | UiColumnType::Surname))
        .map(|column| column.title.to_string())
        .collect()
}

/// Presses `text` with `modifier` (Alt, Control or Shift) held.
pub(crate) fn with_modifier(ui: &Ui, modifier: Key, text: impl Into<SharedString>) {
    let window = ui.session.window.window();
    let held: SharedString = modifier.into();
    window.dispatch_event(WindowEvent::KeyPressed { text: held.clone() });
    key(ui, text);
    window.dispatch_event(WindowEvent::KeyReleased { text: held });
}

pub(crate) fn panel_is_closed(ui: &Ui) -> bool {
    gradebook(ui).get_panel_index() == -1
}

// ---------------------------------------------------------------------------------------------
// Messages on the page that is open, menus and fields from the keyboard

/// The settings folder turns into a file, so nothing the app saves to config.toml can be written.
pub(crate) fn break_saving_the_settings(ui: &Ui) {
    let config_dir = ui.fixture.path("config");
    fs::remove_dir_all(&config_dir).unwrap();
    fs::write(&config_dir, b"in the way").unwrap();
}

/// What the field labelled `label` holds.
pub(crate) fn field_text(ui: &Ui, label: &str) -> String {
    find(ui, label).accessible_value().map(|value| value.to_string()).unwrap_or_default()
}

/// The error on the window, which must also be on screen.
pub(crate) fn error_on_screen(ui: &Ui) -> String {
    let message = ui.session.window.get_error_message().to_string();
    assert!(!message.is_empty(), "there is an error");
    assert!(shows_text(ui, &message), "the error is shown on the open page: {message}");
    message
}

// ---------------------------------------------------------------------------------------------
// Licence screen

/// A database, on a computer whose licence key was never activated.
pub(crate) fn unlicensed() -> Ui {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    Ui::open_unlicensed(fixture)
}

pub(crate) fn licence(ui: &Ui) -> Licence<'_> {
    ui.session.window.global::<Licence>()
}

pub(crate) fn licence_text(notice: &Notice) -> String {
    i18n::licence_notice(Language::English, notice)
}

pub(crate) fn licence_calls(ui: &Ui) -> usize {
    ui.licence.server.calls.load(Ordering::SeqCst)
}

// ---------------------------------------------------------------------------------------------
// Licence while the app runs

/// The class "1A" of `seeded`, on a computer with this licence, in `language`.
pub(crate) fn seeded_with(licence: FakeLicence, language: Language) -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, class, "Ana", "").unwrap();
        let exam = columns::add_column(connection, class, ColumnType::Test, Language::English).unwrap();
        columns::rename_column(connection, exam, "Exam").unwrap();
    }
    state.shutdown();
    let client = licence.client();
    Ui::open_licensed(fixture, language, licence, client)
}

/// The settings page, scrolled down to the Licence card.
pub(crate) fn open_licence_settings(ui: &Ui) {
    open_settings(ui);
    scroll_settings_to(ui, "Check now");
}

// ---------------------------------------------------------------------------------------------
// Updates: the banner and Settings › Updates

pub(crate) fn update(ui: &Ui) -> Update<'_> {
    ui.session.window.global::<Update>()
}

/// A version above this build's.
pub(crate) fn next_version() -> String {
    let mut next = avalio_lite::update::target::current_version();
    next.minor += 1;
    next.patch = 0;
    next.pre = semver::Prerelease::EMPTY;
    next.to_string()
}

/// A release of `version` with this build's file and its proof, at addresses nothing serves.
pub(crate) fn release(version: &str) -> Release {
    let version = semver::Version::parse(version).unwrap();
    let name = crate::common::releases::asset_for(&version);
    let assets = [name.clone(), checksum_name(&name), signature_name(&name)]
        .into_iter()
        .map(|file| Asset { url: format!("https://example.org/{file}"), name: file })
        .collect();
    Release {
        version: version.clone(),
        page: format!("https://github.com/Ran-n/avalio-lite/releases/tag/v{version}"),
        assets,
    }
}

/// What the timer does.
pub(crate) fn automatic_check(ui: &Ui) {
    avalio_lite::update::ui::automatic_check(&ui.session.window, &ui.session.updater);
}

/// The settings page, scrolled down to the Updates card.
pub(crate) fn open_update_settings(ui: &Ui) {
    open_settings(ui);
    scroll_settings_to(ui, "Check for updates now");
}

// ---------------------------------------------------------------------------------------------
// Column widths

/// Where the first mark column is in the sheet: after the first-name and surname columns.
pub(crate) const FIRST_MARK: usize = 2;

/// The width stored for column `index` of class "1A" (`None` for the default), the name columns
/// counted.
pub(crate) fn stored_column_width(ui: &Ui, index: usize) -> Option<i64> {
    let state = ui.session.state.borrow();
    let connection = state.connection().unwrap();
    let class = classes::list_classes(connection).unwrap()[0].id;
    columns::list_columns(connection, class).unwrap()[index].width.map(i64::from)
}

/// The edges that resize the columns, in sheet order: the first-name and surname columns' first.
pub(crate) fn width_edges(ui: &Ui) -> Vec<ElementHandle> {
    elements(ui, "Column width")
}

/// Drags `edge` sideways by `dx` logical pixels with the left button, through real pointer events.
pub(crate) fn drag_edge(edge: &ElementHandle, dx: f32) {
    let start = edge.absolute_position();
    let (x, y) = (start.x + edge.size().width / 2.0, start.y + edge.size().height / 2.0);
    edge.mock_drag(slint::LogicalPosition::new(x + dx, y), PointerEventButton::Left);
}

// ---------------------------------------------------------------------------------------------
// Background work that can't be started

/// A database, with background work that can't be started.
pub(crate) fn with_a_worker_that_cannot_start() -> Ui {
    let ui = Ui::with_database();
    ui.worker.refusing.set(true);
    ui
}
