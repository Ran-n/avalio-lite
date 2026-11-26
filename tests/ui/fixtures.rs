// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.754894
//+ Revised: 	2026/11/23 13:00:31.754894
// -----------------------------------------------------------------------------

//! Fixtures and the Settings navigation the feature modules share: seeded databases, encrypted
//! files, folders and keys.

use crate::common::{Fixture, Ui};
use crate::driver::{PIN, click, elements, find, gradebook, key, shows};
use crate::screens::FIRST_MARK;
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{classes, columns, pupils};
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::Preferences;
use i_slint_backend_testing::ElementHandle;
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};

pub(crate) fn saved_preferences(fixture: &Fixture) -> Preferences {
    fixture.store().load().unwrap().unwrap().preferences
}

/// Class "1A" with pupils Ana and Bea, a test "Exam" worth 100 % out of 10, and a final grade.
pub(crate) fn seeded() -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, class, "Ana", "").unwrap();
        pupils::add_new_pupil(connection, class, "Bea", "").unwrap();
        let exam = columns::add_column(connection, class, ColumnType::Test, Language::English).unwrap();
        columns::rename_column(connection, exam, "Exam").unwrap();
        columns::add_column(connection, class, ColumnType::Weighted, Language::English).unwrap();
    }
    state.shutdown();
    Ui::open(fixture)
}

/// Class "1A" with Ana, and class "2B" with Carla.
pub(crate) fn with_second_class() -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let first = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, first, "Ana", "").unwrap();
        let second = classes::create_class(connection, "2B").unwrap();
        pupils::add_new_pupil(connection, second, "Carla", "").unwrap();
    }
    state.shutdown();
    Ui::open(fixture)
}

/// A fixture whose database is encrypted with `PIN`.
pub(crate) fn encrypted_fixture() -> Fixture {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin(PIN, PIN).unwrap();
    state.shutdown();
    fixture
}

pub(crate) fn encrypted() -> Ui {
    Ui::open(encrypted_fixture())
}

/// What row `row` shows in mark column `column` (counted after the first-name and surname columns).
pub(crate) fn cell_text(ui: &Ui, row: usize, column: usize) -> String {
    let rows = gradebook(ui).get_rows();
    rows.row_data(row).unwrap().cells.row_data(FIRST_MARK + column).unwrap().text.to_string()
}

/// The column header titled `title`, told apart from the name cells labelled the same.
pub(crate) fn header(ui: &Ui, title: &str) -> ElementHandle {
    let found: Vec<ElementHandle> = elements(ui, title)
        .into_iter()
        .filter(|element| element.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button))
        .collect();
    assert_eq!(found.len(), 1, "expected one header {title:?}");
    found.into_iter().next().unwrap()
}

/// The text fields labelled `label`, without the column header of the same name: the name cells
/// are labelled like their column ("First name", "Surname").
pub(crate) fn name_fields(ui: &Ui, label: &str) -> Vec<ElementHandle> {
    elements(ui, label)
        .into_iter()
        .filter(|element| element.accessible_role() != Some(i_slint_backend_testing::AccessibleRole::Button))
        .collect()
}

pub(crate) fn type_pin_on_keypad(ui: &Ui, pin: &str) {
    for digit in pin.chars() {
        click(ui, &digit.to_string());
    }
}

pub(crate) fn unlock(ui: &Ui) {
    type_pin_on_keypad(ui, PIN);
    click(ui, "Unlock");
    assert!(!ui.session.window.get_locked());
}

pub(crate) fn open_settings(ui: &Ui) {
    click(ui, "Settings");
    assert!(shows(ui, "Galego"), "the settings page is showing");
}

/// The settings page, scrolled from the top a step at a time until a control labelled `label`
/// is on screen (the test backend only sees what is on screen).
pub(crate) fn scroll_settings_to(ui: &Ui, label: &str) {
    scroll_settings_until(ui, label, |ui| shows(ui, label));
}

/// The settings page, scrolled the same way until `found` holds; `what` names it in the failure.
pub(crate) fn scroll_settings_until(ui: &Ui, what: &str, found: impl Fn(&Ui) -> bool) {
    // a point on the page (right of the sidebar) that stays on it however far it has scrolled
    let size = ui.session.window.window().size().to_logical(ui.session.window.window().scale_factor());
    let position = slint::LogicalPosition::new(size.width * 0.6, size.height / 2.0);
    let wheel = |delta_y: f32| {
        ui.session.window.window().dispatch_event(WindowEvent::PointerScrolled { position, delta_x: 0.0, delta_y });
    };
    wheel(10_000.0);
    for _ in 0..80 {
        if found(ui) {
            // and a little further, so the whole control is on screen to be clicked
            wheel(-120.0);
            assert!(found(ui), "{what:?} stays on screen");
            return;
        }
        wheel(-60.0);
    }
    panic!("scrolled the whole settings page without finding {what:?}");
}

/// The settings page, scrolled down to the PIN card (the shortcuts card sits above it).
pub(crate) fn open_pin_settings(ui: &Ui) {
    open_settings(ui);
    find(ui, "Galego").scroll(0.0, -380.0);
    assert!(shows(ui, "Repeat PIN"), "scrolled to the PIN card");
}

/// The settings page, scrolled until the Database, Backup and Logs cards are all on screen: both
/// Move… buttons show, so the Backup card between them does too.
pub(crate) fn scroll_to_the_folder_cards(ui: &Ui) {
    scroll_settings_until(ui, "the Database, Backup and Logs cards", |ui| elements(ui, "Move…").len() == 2);
}

/// The settings page, scrolled down to the database and log folders.
pub(crate) fn open_folder_settings(ui: &Ui) {
    open_settings(ui);
    scroll_to_the_folder_cards(ui);
}

// ---------------------------------------------------------------------------------------------
// Keyboard shortcuts

/// Holds Ctrl (and Shift when asked) while pressing `text`.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "test helper: Shift held or not with Ctrl, as the shortcut under test needs"
)]
pub(crate) fn with_keys(ui: &Ui, shift: bool, text: impl Into<SharedString>) {
    let window = ui.session.window.window();
    let held: Vec<SharedString> =
        [Some(Key::Control), shift.then_some(Key::Shift)].into_iter().flatten().map(Into::into).collect();
    for modifier in &held {
        window.dispatch_event(WindowEvent::KeyPressed { text: modifier.clone() });
    }
    key(ui, text);
    for modifier in held.iter().rev() {
        window.dispatch_event(WindowEvent::KeyReleased { text: modifier.clone() });
    }
}

pub(crate) fn with_shortcut_config(fixture: &Fixture, entries: &[(&str, &str)]) {
    let mut config = fixture.store().load().unwrap().unwrap();
    for (name, keys) in entries {
        config.shortcuts.insert((*name).to_owned(), (*keys).to_owned());
    }
    fixture.store().save(&config).unwrap();
}
