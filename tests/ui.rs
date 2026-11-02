// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:06.576798
//+ Revised: 	2026/11/02 14:00:09.863535
// -----------------------------------------------------------------------------

//! The real window, wired as in the app, driven on Slint's headless backend: elements are found
//! by their accessible labels and clicked with pointer events, and text is typed as key events.
//! Dialogs are answered by `FakeDialogs`; data is checked in the database the app has open.

mod common;

use std::fs;

use avalio_lite::app::dialogs::FileKind;
use avalio_lite::backup;
use avalio_lite::config::{DATABASE_FILE_NAME, LOG_FILE_NAME, Preferences, ThemeChoice};
use avalio_lite::db;
use avalio_lite::error::{AppError, Folder};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{classes, columns, pupils};
use avalio_lite::i18n::{self, Language, Message};
use avalio_lite::ui::{Gradebook, Section, SetupNotice, Theme, Tone};
use common::controls;
use common::{Fixture, Ui};
use i_slint_backend_testing::ElementHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};

const PIN: &str = "2468";

// ---------------------------------------------------------------------------------------------
// Driving the window

fn elements(ui: &Ui, label: &str) -> Vec<ElementHandle> {
    controls::labelled(&ui.session.window, label)
}

fn find(ui: &Ui, label: &str) -> ElementHandle {
    let found = elements(ui, label);
    assert_eq!(found.len(), 1, "expected one control labelled {label:?}, found {}", found.len());
    found.into_iter().next().unwrap()
}

fn shows(ui: &Ui, label: &str) -> bool {
    !elements(ui, label).is_empty()
}

/// Any visible text (plain `Text` elements included) equal to `text`.
fn shows_text(ui: &Ui, text: &str) -> bool {
    ElementHandle::find_by_accessible_label(&ui.session.window, text).next().is_some()
}

fn click_element(element: &ElementHandle) {
    element.mock_single_click(PointerEventButton::Left);
}

fn click(ui: &Ui, label: &str) {
    click_element(&find(ui, label));
}

/// A menu option, told apart from a column header with the same title by its hint.
fn click_menu_option(ui: &Ui, title: &str) {
    let options: Vec<ElementHandle> = elements(ui, title)
        .into_iter()
        .filter(|element| element.accessible_description().is_some_and(|hint| !hint.is_empty()))
        .collect();
    assert_eq!(options.len(), 1, "expected one menu option {title:?}");
    click_element(&options[0]);
}

fn click_nth(ui: &Ui, label: &str, index: usize) {
    click_element(&elements(ui, label)[index]);
}

fn key(ui: &Ui, text: impl Into<SharedString>) {
    let text = text.into();
    let window = ui.session.window.window();
    window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text });
}

fn with_control(ui: &Ui, text: &str) {
    let window = ui.session.window.window();
    let control: SharedString = Key::Control.into();
    window.dispatch_event(WindowEvent::KeyPressed { text: control.clone() });
    key(ui, text);
    window.dispatch_event(WindowEvent::KeyReleased { text: control });
}

fn type_text(ui: &Ui, text: &str) {
    for character in text.chars() {
        key(ui, character.to_string());
    }
}

/// Selects everything in the focused field and types `text` over it.
fn replace_text(ui: &Ui, text: &str) {
    with_control(ui, "a");
    if text.is_empty() {
        key(ui, Key::Backspace);
    }
    type_text(ui, text);
}

/// Clicks the field labelled `label` and types `text` over what is in it.
fn type_into(ui: &Ui, label: &str, text: &str) {
    click(ui, label);
    replace_text(ui, text);
}

fn press_enter(ui: &Ui) {
    key(ui, Key::Return);
}

fn gradebook(ui: &Ui) -> Gradebook<'_> {
    ui.session.window.global::<Gradebook>()
}

fn english(message: &Message) -> String {
    i18n::text(Language::English, message)
}

fn error_text(error: &AppError) -> String {
    i18n::error_message(Language::English, error)
}

// ---------------------------------------------------------------------------------------------
// Data

fn count(ui: &Ui, sql: &str) -> i64 {
    ui.session.state.borrow().connection().unwrap().query_row(sql, [], |row| row.get(0)).unwrap()
}

fn saved_preferences(fixture: &Fixture) -> Preferences {
    fixture.store().load().unwrap().unwrap().preferences
}

/// Class "1A" with pupils Ana and Bea, a test "Exam" worth 100 % out of 10, and a final grade.
fn seeded() -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, class, "Ana").unwrap();
        pupils::add_new_pupil(connection, class, "Bea").unwrap();
        let exam = columns::add_column(connection, class, ColumnType::Test, Language::English).unwrap();
        columns::rename_column(connection, exam, "Exam").unwrap();
        columns::add_column(connection, class, ColumnType::Weighted, Language::English).unwrap();
    }
    state.shutdown();
    Ui::open(fixture)
}

/// Class "1A" with Ana, and class "2B" with Carla.
fn with_second_class() -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let first = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, first, "Ana").unwrap();
        let second = classes::create_class(connection, "2B").unwrap();
        pupils::add_new_pupil(connection, second, "Carla").unwrap();
    }
    state.shutdown();
    Ui::open(fixture)
}

/// A fixture whose database is encrypted with `PIN`.
fn encrypted_fixture() -> Fixture {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin(PIN, PIN).unwrap();
    state.shutdown();
    fixture
}

fn encrypted() -> Ui {
    Ui::open(encrypted_fixture())
}

/// Starts like an SQLite file, so it isn't taken for an encrypted one, but the rest is garbage.
fn corrupt_database() -> Vec<u8> {
    let mut bytes = b"SQLite format 3\0".to_vec();
    bytes.extend(std::iter::repeat_n(0xA5, 4096));
    bytes
}

fn cell_text(ui: &Ui, row: usize, column: usize) -> String {
    let rows = gradebook(ui).get_rows();
    rows.row_data(row).unwrap().cells.row_data(column).unwrap().text.to_string()
}

fn type_pin_on_keypad(ui: &Ui, pin: &str) {
    for digit in pin.chars() {
        click(ui, &digit.to_string());
    }
}

fn unlock(ui: &Ui) {
    type_pin_on_keypad(ui, PIN);
    click(ui, "Unlock");
    assert!(!ui.session.window.get_locked());
}

fn open_settings(ui: &Ui) {
    click(ui, "Settings");
    assert!(shows(ui, "Galego"), "the settings page is showing");
}

/// The settings page, scrolled down to the PIN card (the shortcuts card sits above it).
fn open_pin_settings(ui: &Ui) {
    open_settings(ui);
    find(ui, "Galego").scroll(0.0, -320.0);
    assert!(shows(ui, "Repeat PIN"), "scrolled to the PIN card");
}

/// The settings page, scrolled down to the database and log folders.
fn open_folder_settings(ui: &Ui) {
    open_settings(ui);
    find(ui, "Galego").scroll(0.0, -1200.0);
    assert!(shows(ui, "Open another database…"), "scrolled to the folders");
}

// ---------------------------------------------------------------------------------------------
// Startup

#[test]
fn first_run_shows_setup_with_the_suggested_folder() {
    let ui = Ui::open(Fixture::new());
    let window = &ui.session.window;
    assert!(window.get_setup_required());
    assert_eq!(window.get_setup_notice(), SetupNotice::FirstRun);
    assert!(!window.get_setup_database_folder().is_empty());
    assert_eq!(window.get_setup_database_folder(), window.get_setup_log_folder());
    assert!(shows(&ui, "Create database"));
}

#[test]
fn an_existing_database_opens_on_its_first_class() {
    let ui = seeded();
    assert!(!ui.session.window.get_setup_required());
    assert!(!ui.session.window.get_locked());
    assert_eq!(gradebook(&ui).get_class_name(), "1A");
    assert!(shows(&ui, "Exam"));
    assert_eq!(gradebook(&ui).get_rows().row_count(), 2);
}

#[test]
fn a_missing_database_asks_where_it_went() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    fs::remove_file(fixture.database()).unwrap();
    let ui = Ui::open(fixture);
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_setup_notice(), SetupNotice::DatabaseMissing);
    assert_eq!(ui.session.window.get_setup_notice_detail(), ui.fixture.database().display().to_string());
}

#[test]
fn unreadable_settings_ask_for_setup_again() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.path("config")).unwrap();
    fs::write(fixture.store().path(), "not = [valid").unwrap();
    let ui = Ui::open(fixture);
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_setup_notice(), SetupNotice::ConfigUnreadable);
}

#[test]
fn an_unusable_database_asks_for_another() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    fs::write(fixture.database(), corrupt_database()).unwrap();
    let ui = Ui::open(fixture);
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_setup_notice(), SetupNotice::DatabaseUnusable);
}

#[test]
fn an_encrypted_database_starts_on_the_lock_screen() {
    let ui = encrypted();
    assert!(ui.session.window.get_locked());
    assert!(ui.session.window.get_pin_set());
    assert!(shows(&ui, "Unlock"));
    assert!(ui.session.state.borrow().connection().is_none(), "nothing is readable before the PIN");
}

#[test]
fn the_saved_language_and_theme_are_applied_at_startup() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_language(Language::Galician).unwrap();
    state.set_theme(ThemeChoice::Dark).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    assert_eq!(ui.session.window.get_language_code(), "gl");
    assert_eq!(i18n::current(), Language::Galician);
    assert!(ui.session.window.global::<Theme>().get_dark());
    assert!(shows(&ui, "Crear clase"), "the interface is in Galician");
}

#[test]
fn a_saved_light_theme_wins_over_the_windows_setting() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_theme(ThemeChoice::Light).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    assert!(!ui.session.window.global::<Theme>().get_dark());
}

#[test]
fn the_windows_language_is_used_until_one_is_chosen() {
    let ui = Ui::open_in(Fixture::new(), Language::Galician);
    assert_eq!(ui.session.window.get_language_code(), "gl");
    assert!(shows(&ui, "Crear base de datos"));
}

// ---------------------------------------------------------------------------------------------
// First-run setup

#[test]
fn first_run_creates_the_database_in_the_chosen_folders() {
    let ui = Ui::open(Fixture::new());
    type_into(&ui, "Database folder", &ui.fixture.path("data").display().to_string());
    type_into(&ui, "Log folder", &ui.fixture.path("logs").display().to_string());
    click(&ui, "Create database");
    assert!(!ui.session.window.get_setup_required());
    assert!(ui.fixture.database().is_file());
    assert!(ui.fixture.path("logs").join(LOG_FILE_NAME).is_file());
    assert_eq!(ui.session.window.get_database_path(), ui.fixture.database().display().to_string());
    assert!(shows(&ui, "Create class"));
}

#[test]
fn browse_fills_each_folder_field_from_the_picker() {
    let ui = Ui::open(Fixture::new());
    let data = ui.fixture.path("data");
    let logs = ui.fixture.path("logs");
    let suggested = ui.session.window.get_setup_database_folder().to_string();
    ui.dialogs.pick_folder_next(Some(&data));
    ui.dialogs.pick_folder_next(Some(&logs));
    click_nth(&ui, "Browse…", 0);
    click_nth(&ui, "Browse…", 1);
    assert_eq!(ui.session.window.get_setup_database_folder(), data.display().to_string());
    assert_eq!(ui.session.window.get_setup_log_folder(), logs.display().to_string());
    assert_eq!(ui.dialogs.folder_starts.borrow()[0].display().to_string(), suggested, "opens at the current folder");
}

#[test]
fn a_cancelled_browse_keeps_the_folder() {
    let ui = Ui::open(Fixture::new());
    let before = ui.session.window.get_setup_database_folder();
    ui.dialogs.pick_folder_next(None);
    ui.dialogs.pick_folder_next(None);
    click_nth(&ui, "Browse…", 0);
    click_nth(&ui, "Browse…", 1);
    assert_eq!(ui.session.window.get_setup_database_folder(), before);
    assert_eq!(ui.session.window.get_setup_log_folder(), before);
}

#[test]
fn an_empty_folder_is_refused_on_the_setup_screen() {
    let ui = Ui::open(Fixture::new());
    type_into(&ui, "Database folder", "");
    click(&ui, "Create database");
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::EmptyPath(Folder::Database)));
    assert!(ui.fixture.store().load().unwrap().is_none(), "nothing is saved");
}

#[test]
fn setup_can_open_an_existing_database() {
    let other = Fixture::new();
    other.set_up().shutdown();
    let ui = Ui::open(Fixture::new());
    type_into(&ui, "Log folder", &ui.fixture.path("logs").display().to_string());
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open an existing database…");
    assert!(!ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_database_path(), other.database().display().to_string());
}

#[test]
fn a_cancelled_open_stays_on_setup() {
    let ui = Ui::open(Fixture::new());
    ui.dialogs.pick_file_next(None);
    click(&ui, "Open an existing database…");
    assert!(ui.session.window.get_setup_required());
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn a_file_that_isnt_a_database_is_reported_on_setup() {
    let ui = Ui::open(Fixture::new());
    let not_a_database = ui.fixture.path("broken.db");
    fs::write(&not_a_database, corrupt_database()).unwrap();
    type_into(&ui, "Log folder", &ui.fixture.path("logs").display().to_string());
    ui.dialogs.pick_file_next(Some(&not_a_database));
    click(&ui, "Open an existing database…");
    assert!(ui.session.window.get_setup_required());
    assert!(!ui.session.window.get_error_message().is_empty());
    assert!(ui.fixture.store().load().unwrap().is_none(), "nothing is saved");
}

#[test]
fn opening_an_encrypted_database_from_setup_asks_for_its_pin() {
    let other = encrypted_fixture();
    let ui = Ui::open(Fixture::new());
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open an existing database…");
    assert!(ui.session.window.get_locked());
    unlock(&ui);
}

// ---------------------------------------------------------------------------------------------
// Lock screen

#[test]
fn the_keypad_unlocks_with_the_right_pin() {
    let ui = encrypted();
    type_pin_on_keypad(&ui, PIN);
    assert_eq!(ui.session.window.get_unlock_pin(), PIN);
    click(&ui, "Unlock");
    assert!(!ui.session.window.get_locked());
    assert!(ui.session.state.borrow().connection().is_some());
    assert!(ui.session.window.get_unlock_pin().is_empty(), "the PIN isn't kept in the field");
}

#[test]
fn typing_the_pin_and_enter_unlocks() {
    let ui = encrypted();
    type_text(&ui, PIN);
    press_enter(&ui);
    assert!(!ui.session.window.get_locked());
}

#[test]
fn erase_removes_the_last_digit() {
    let ui = encrypted();
    type_pin_on_keypad(&ui, "123");
    click(&ui, "Erase");
    assert_eq!(ui.session.window.get_unlock_pin(), "12");
    for _ in 0..3 {
        click(&ui, "Erase");
    }
    assert_eq!(ui.session.window.get_unlock_pin(), "", "erasing an empty PIN does nothing");
}

#[test]
fn a_wrong_pin_is_reported_and_keeps_the_lock() {
    let ui = encrypted();
    type_pin_on_keypad(&ui, "1111");
    click(&ui, "Unlock");
    assert!(ui.session.window.get_locked());
    assert_eq!(ui.session.window.get_unlock_error(), error_text(&AppError::WrongPin));
    assert!(ui.session.window.get_unlock_pin().is_empty());
}

/// The leave and return callbacks only act when they apply: no database chosen, or none locked.
#[test]
fn leaving_and_returning_to_the_lock_screen_need_a_locked_database() {
    let ui = Ui::open(Fixture::new());
    ui.session.window.invoke_forgot_pin();
    assert_eq!(ui.session.window.get_setup_notice(), SetupNotice::FirstRun, "still the first-run screen");
    drop(ui);
    let ui = seeded();
    ui.session.window.invoke_back_to_lock();
    assert!(!ui.session.window.get_locked(), "nothing is locked, so there is nothing to go back to");
}

/// Was: the lock screen said the file doesn't exist on every try, with no way off it.
#[test]
fn regression_a_database_gone_while_locked_asks_where_it_went() {
    let ui = encrypted();
    fs::remove_file(ui.fixture.database()).unwrap();
    type_pin_on_keypad(&ui, PIN);
    click(&ui, "Unlock");
    let window = &ui.session.window;
    assert!(window.get_setup_required(), "the setup screen replaces the lock screen");
    assert_eq!(window.get_setup_notice(), SetupNotice::DatabaseMissing);
    assert_eq!(window.get_setup_notice_detail(), ui.fixture.database().display().to_string());
    assert!(!ui.fixture.database().exists(), "no empty database was made in its place");
    assert!(shows(&ui, "Create database"));
}

#[test]
fn forgot_the_pin_leaves_the_lock_screen_for_setup_and_can_come_back() {
    let ui = encrypted();
    click(&ui, "Forgot the PIN?");
    let window = &ui.session.window;
    assert!(window.get_setup_required());
    assert_eq!(window.get_setup_notice(), SetupNotice::LeftLocked);
    assert_eq!(window.get_setup_notice_detail(), ui.fixture.database().display().to_string());
    assert!(db::is_encrypted(&ui.fixture.database()).unwrap(), "the locked file is left as it is");
    click(&ui, "Back to the lock screen");
    assert!(!window.get_setup_required());
    assert!(window.get_locked());
    unlock(&ui);
}

#[test]
fn after_forgetting_the_pin_a_new_database_can_be_started_elsewhere() {
    let ui = encrypted();
    click(&ui, "Forgot the PIN?");
    let window = &ui.session.window;
    window.set_setup_database_folder(ui.fixture.path("fresh").display().to_string().into());
    window.set_setup_log_folder(ui.fixture.path("logs").display().to_string().into());
    click(&ui, "Create database");
    assert!(!window.get_setup_required());
    assert!(!window.get_locked(), "the new database has no PIN");
    assert!(!window.get_pin_set());
    let saved = ui.fixture.store().load().unwrap().unwrap();
    assert_eq!(saved.database_path, ui.fixture.path("fresh").join(DATABASE_FILE_NAME));
    assert!(db::is_encrypted(&ui.fixture.database()).unwrap(), "the old file is still there, still encrypted");
}

#[test]
fn back_on_the_lock_screen_does_not_reset_the_wait() {
    let ui = encrypted();
    for _ in 0..5 {
        type_pin_on_keypad(&ui, "1111");
        click(&ui, "Unlock");
    }
    click(&ui, "Forgot the PIN?");
    click(&ui, "Back to the lock screen");
    type_pin_on_keypad(&ui, PIN);
    click(&ui, "Unlock");
    assert!(ui.session.window.get_locked(), "still waiting");
    let error = ui.session.window.get_unlock_error().to_string();
    let waits: Vec<String> = (1..=30).map(|seconds| error_text(&AppError::PinWait { seconds })).collect();
    assert!(waits.contains(&error), "the wait goes on: {error}");
}

#[test]
fn after_five_wrong_pins_even_the_right_one_waits() {
    let ui = encrypted();
    for _ in 0..5 {
        type_pin_on_keypad(&ui, "1111");
        click(&ui, "Unlock");
    }
    let error = ui.session.window.get_unlock_error().to_string();
    assert_eq!(error, error_text(&AppError::PinWait { seconds: 30 }), "the fifth miss starts a wait");
    type_pin_on_keypad(&ui, PIN);
    click(&ui, "Unlock");
    assert!(ui.session.window.get_locked(), "no tries while waiting");
}

#[test]
fn the_sidebar_lock_and_ctrl_l_lock_an_encrypted_database() {
    let ui = encrypted();
    unlock(&ui);
    click(&ui, "Lock (Ctrl+L)");
    assert!(ui.session.window.get_locked());
    assert!(ui.session.state.borrow().connection().is_none());

    unlock(&ui);
    with_control(&ui, "l");
    assert!(ui.session.window.get_locked(), "Ctrl+L locks too");
}

#[test]
fn ctrl_l_without_a_pin_explains_how_to_lock() {
    let ui = seeded();
    assert!(!shows(&ui, "Lock (Ctrl+L)"), "no lock button without a PIN");
    with_control(&ui, "l");
    assert!(!ui.session.window.get_locked());
    assert_eq!(ui.session.window.get_error_message(), english(&Message::LockNeedsPin));
}

// ---------------------------------------------------------------------------------------------
// Keyboard shortcuts

/// Holds Ctrl (and Shift when asked) while pressing `text`.
fn with_keys(ui: &Ui, shift: bool, text: impl Into<SharedString>) {
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

fn with_shortcut_config(fixture: &Fixture, entries: &[(&str, &str)]) {
    let mut config = fixture.store().load().unwrap().unwrap();
    for (name, keys) in entries {
        config.shortcuts.insert((*name).to_owned(), (*keys).to_owned());
    }
    fixture.store().save(&config).unwrap();
}

#[test]
fn every_shortcut_works_from_a_focused_cell() {
    let ui = with_second_class();
    let window = &ui.session.window;
    click(&ui, "Pupil");
    with_keys(&ui, true, "D");
    assert!(window.global::<Theme>().get_dark(), "Ctrl+Shift+D switches to dark");
    assert_eq!(saved_preferences(&ui.fixture).theme, Some(ThemeChoice::Dark), "and saves it");
    with_keys(&ui, false, "h");
    assert!(!gradebook(&ui).get_show_column_settings(), "Ctrl+H hides weights and scales");
    with_keys(&ui, false, ",");
    assert_eq!(window.get_section(), Section::Settings, "Ctrl+, opens Settings");
    with_keys(&ui, false, ",");
    assert_eq!(window.get_section(), Section::Sheet, "and closes it again");
    assert_eq!(gradebook(&ui).get_class_name(), "1A");
    with_keys(&ui, false, Key::PageDown);
    assert_eq!(gradebook(&ui).get_class_name(), "2B", "Ctrl+PageDown: the next class");
    with_keys(&ui, false, Key::PageDown);
    assert_eq!(gradebook(&ui).get_class_name(), "1A", "round to the first again");
    with_keys(&ui, false, ",");
    with_keys(&ui, false, Key::PageUp);
    assert_eq!(gradebook(&ui).get_class_name(), "2B", "Ctrl+PageUp from Settings: the previous class");
    assert_eq!(window.get_section(), Section::Sheet, "shown on the sheet");
}

#[test]
fn the_class_shortcuts_do_nothing_without_classes() {
    let ui = Ui::with_database();
    with_keys(&ui, false, Key::PageDown);
    with_keys(&ui, false, Key::PageUp);
    assert_eq!(gradebook(&ui).get_current_class_id(), -1);
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn shortcuts_do_nothing_on_the_lock_and_first_run_screens() {
    let ui = encrypted();
    with_keys(&ui, false, ",");
    with_keys(&ui, true, "D");
    assert!(ui.session.window.get_locked());
    assert!(!ui.session.window.global::<Theme>().get_dark());
    type_text(&ui, "12");
    assert_eq!(ui.session.window.get_unlock_pin(), "12", "the PIN field still takes digits");
    drop(ui);

    let ui = Ui::open(Fixture::new());
    with_keys(&ui, false, ",");
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_section(), Section::Sheet);
}

#[test]
fn a_configured_lock_key_locks_and_shows_in_the_labels() {
    let fixture = encrypted_fixture();
    with_shortcut_config(&fixture, &[("lock", "Ctrl+K")]);
    let ui = Ui::open(fixture);
    unlock(&ui);
    assert!(shows(&ui, "Lock (Ctrl+K)"));
    open_pin_settings(&ui);
    let pin_text = "The database file is encrypted with your PIN. The app asks for it when it opens; lock it any time with the Lock button or Ctrl+K.";
    assert!(shows_text(&ui, pin_text));
    with_control(&ui, "l");
    assert!(!ui.session.window.get_locked(), "Ctrl+L no longer locks");
    with_control(&ui, "k");
    assert!(ui.session.window.get_locked());
}

#[test]
fn bad_shortcut_entries_are_reported_and_use_the_defaults() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    with_shortcut_config(&fixture, &[("lock", "Ctrl+"), ("toggle-theme", "Ctrl+L")]);
    let ui = Ui::open(fixture);
    let banner = ui.session.window.get_error_message().to_string();
    assert!(banner.contains("lock = \"Ctrl+\"") && banner.contains("toggle-theme = \"Ctrl+L\""), "{banner}");
    with_keys(&ui, true, "D");
    assert!(ui.session.window.global::<Theme>().get_dark(), "the theme keeps its default");
}

#[test]
fn settings_lists_the_shortcuts_in_the_current_language() {
    let ui = seeded();
    open_settings(&ui);
    assert!(shows_text(&ui, "Keyboard shortcuts"));
    let items = ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_items();
    assert_eq!(items.row_count(), 6);
    let theme = items.row_data(1).unwrap();
    assert_eq!(theme.label, "Switch between light and dark");
    assert_eq!(theme.keys.iter().collect::<Vec<_>>(), ["Ctrl", "Shift", "D"]);
    click(&ui, "Galego");
    let theme = ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_items().row_data(1).unwrap();
    assert_eq!(theme.label, "Cambiar entre claro e escuro");
    assert_eq!(theme.keys.iter().collect::<Vec<_>>(), ["Ctrl", "Maiús", "D"]);
    assert!(shows_text(&ui, "Atallos de teclado"));
}

fn shortcut_row(ui: &Ui, row: usize) -> avalio_lite::ui::ShortcutItem {
    ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_items().row_data(row).unwrap()
}

fn recording(ui: &Ui) -> i32 {
    ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_recording()
}

#[test]
fn a_shortcut_is_changed_by_pressing_change_and_the_new_keys() {
    let ui = seeded();
    open_settings(&ui);
    assert!(ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_all_default());
    click_nth(&ui, "Change", 0);
    assert_eq!(recording(&ui), 0, "the Lock row records");
    assert!(shows_text(&ui, "Press the new keys…"));
    with_control(&ui, "k");
    assert_eq!(recording(&ui), -1);
    assert_eq!(shortcut_row(&ui, 0).keys.iter().collect::<Vec<_>>(), ["Ctrl", "K"]);
    assert!(!shortcut_row(&ui, 0).is_default);
    assert_eq!(ui.session.window.get_status_message(), "Lock is now Ctrl+K.");
    assert_eq!(saved_shortcut(&ui, "lock"), Some("Ctrl+K".to_owned()), "saved at once");
    assert!(!ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_all_default(), "Reset all is enabled");
}

fn saved_shortcut(ui: &Ui, name: &str) -> Option<String> {
    ui.fixture.store().load().unwrap().unwrap().shortcuts.get(name).cloned()
}

#[test]
fn refused_keys_keep_the_row_recording_with_the_reason() {
    let ui = seeded();
    open_settings(&ui);
    click_nth(&ui, "Change", 0);
    with_control(&ui, "h");
    assert_eq!(recording(&ui), 0, "still recording after a clash");
    assert_eq!(ui.session.window.get_error_message(), "Ctrl+H is already used by Show or hide weights and scales.");
    key(&ui, "k");
    assert_eq!(ui.session.window.get_error_message(), "Add Ctrl or Alt: a plain letter would type into the cell.");
    with_control(&ui, "c");
    assert_eq!(ui.session.window.get_error_message(), "Ctrl+C is used for editing text.");
    key(&ui, Key::Control);
    assert_eq!(recording(&ui), 0, "Ctrl alone waits for the key");
    key(&ui, Key::Escape);
    assert_eq!(recording(&ui), -1, "Esc cancels");
    assert!(shortcut_row(&ui, 0).is_default, "and the shortcut is unchanged");
    assert_eq!(saved_shortcut(&ui, "lock"), None);
}

#[test]
fn leaving_settings_or_locking_ends_a_recording() {
    let ui = encrypted();
    unlock(&ui);
    type_into(&ui, "New class", "1A");
    press_enter(&ui);
    open_settings(&ui);
    click_nth(&ui, "Change", 1);
    with_control(&ui, "l");
    assert!(!ui.session.window.get_locked(), "while recording, Ctrl+L is recorded (and refused), not run");
    click(&ui, "1A");
    assert_eq!(recording(&ui), -1, "leaving Settings cancels");
    open_settings(&ui);
    click_nth(&ui, "Change", 1);
    click(&ui, "Lock (Ctrl+L)");
    assert_eq!(recording(&ui), -1, "locking cancels");
}

#[test]
fn reset_and_reset_all_bring_back_the_defaults() {
    let ui = seeded();
    open_settings(&ui);
    click_nth(&ui, "Change", 0);
    with_control(&ui, "k");
    click_nth(&ui, "Change", 4);
    key(&ui, Key::F8);
    assert_eq!(saved_shortcut(&ui, "next-class"), Some("F8".to_owned()));
    assert_eq!(elements(&ui, "Reset").len(), 2, "a Reset for each changed row");
    click_nth(&ui, "Reset", 0);
    assert!(shortcut_row(&ui, 0).is_default);
    assert_eq!(saved_shortcut(&ui, "lock"), None);
    find(&ui, "Galego").scroll(0.0, -200.0);
    click(&ui, "Reset all");
    assert_eq!(ui.session.window.get_status_message(), "All keyboard shortcuts are back to their defaults.");
    assert!(ui.fixture.store().load().unwrap().unwrap().shortcuts.is_empty());
    assert!(ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_all_default(), "Reset all is greyed again");
    assert!(!shows(&ui, "Reset"));
}

#[test]
fn a_recording_for_a_row_that_does_not_exist_changes_nothing() {
    let ui = seeded();
    ui.session.window.global::<avalio_lite::ui::Shortcuts>().invoke_record(99, "k".into(), true, false, false);
    ui.session.window.global::<avalio_lite::ui::Shortcuts>().invoke_reset(-1);
    assert!(ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_all_default());
    assert!(ui.fixture.store().load().unwrap().unwrap().shortcuts.is_empty());
}

#[test]
fn a_reset_that_would_clash_names_the_action_with_those_keys() {
    let ui = seeded();
    open_settings(&ui);
    click_nth(&ui, "Change", 0);
    with_control(&ui, "k");
    click_nth(&ui, "Change", 2);
    with_control(&ui, "l");
    assert_eq!(shortcut_row(&ui, 2).keys.iter().collect::<Vec<_>>(), ["Ctrl", "L"]);
    click_nth(&ui, "Reset", 0);
    assert_eq!(ui.session.window.get_error_message(), "Ctrl+L is already used by Show or hide weights and scales.");
}

// ---------------------------------------------------------------------------------------------
// Settings: PIN

#[test]
fn setting_a_pin_encrypts_the_database() {
    let ui = seeded();
    open_pin_settings(&ui);
    type_into(&ui, "PIN", PIN);
    type_into(&ui, "Repeat PIN", PIN);
    click(&ui, "Encrypt with this PIN");
    assert!(ui.session.window.get_pin_set());
    assert_eq!(ui.session.window.get_status_message(), english(&Message::PinTurnedOn));
    assert!(ui.session.window.get_pin_new().is_empty(), "the fields are cleared");
    assert!(ui.session.state.borrow().is_protected());
    assert!(shows(&ui, "Lock (Ctrl+L)"));
}

#[test]
fn a_password_with_letters_and_spaces_locks_and_unlocks() {
    const PASSWORD: &str = "corvo azul 7";
    let ui = seeded();
    open_pin_settings(&ui);
    type_into(&ui, "PIN", PASSWORD);
    type_into(&ui, "Repeat PIN", PASSWORD);
    click(&ui, "Encrypt with this PIN");
    assert!(ui.session.window.get_pin_set(), "{}", ui.session.window.get_error_message());
    click(&ui, "Lock (Ctrl+L)");
    assert!(ui.session.window.get_locked());
    type_text(&ui, "corvo azul 8");
    press_enter(&ui);
    assert!(ui.session.window.get_locked(), "a near miss doesn't open it");
    type_text(&ui, PASSWORD);
    press_enter(&ui);
    assert!(!ui.session.window.get_locked());
    assert!(ui.session.state.borrow().connection().is_some());
}

#[test]
fn a_password_can_be_clicked_on_the_letter_keys() {
    const PASSWORD: &str = "Ñu 7xa";
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin(PASSWORD, PASSWORD).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    click(&ui, "Letters");
    assert!(!shows(&ui, "1"), "the letter keys replace the number keys");
    click(&ui, "Capital letters");
    click(&ui, "Ñ");
    click(&ui, "Capital letters");
    click(&ui, "u");
    click(&ui, "Space");
    click(&ui, "Numbers");
    click(&ui, "7");
    click(&ui, "Letters");
    for letter in ["x", "q", "Erase", "a"] {
        click(&ui, letter);
    }
    assert_eq!(ui.session.window.get_unlock_pin(), PASSWORD);
    click(&ui, "Unlock");
    assert!(!ui.session.window.get_locked());
    assert!(ui.session.state.borrow().connection().is_some());
}

#[test]
fn a_password_with_symbols_can_be_clicked_on_the_symbol_keys() {
    const PASSWORD: &str = "a#\\€¿b";
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin(PASSWORD, PASSWORD).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    click(&ui, "Letters");
    click(&ui, "a");
    click(&ui, "Symbols");
    assert!(!shows(&ui, "q"), "the symbol keys replace the letter keys");
    click(&ui, "#");
    click(&ui, "\\");
    click(&ui, "More symbols");
    assert!(!shows(&ui, "#"), "the second page of symbols");
    click(&ui, "€");
    click(&ui, "¿");
    click(&ui, "Letters");
    click(&ui, "b");
    assert_eq!(ui.session.window.get_unlock_pin(), PASSWORD);
    click(&ui, "Unlock");
    assert!(!ui.session.window.get_locked());
}

#[test]
fn the_lock_screen_opens_on_the_number_keys_again() {
    let ui = encrypted();
    click(&ui, "Letters");
    assert!(shows(&ui, "q"));
    type_text(&ui, PIN);
    press_enter(&ui);
    assert!(!ui.session.window.get_locked(), "the keyboard still types into the PIN field");
    click(&ui, "Lock (Ctrl+L)");
    assert!(shows(&ui, "1") && !shows(&ui, "q"), "a new lock screen starts on the numbers");
}

#[test]
fn mismatched_pins_are_refused() {
    let ui = seeded();
    open_pin_settings(&ui);
    type_into(&ui, "PIN", PIN);
    type_into(&ui, "Repeat PIN", "1357");
    click(&ui, "Encrypt with this PIN");
    assert!(!ui.session.window.get_pin_set());
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::PinMismatch));
    assert!(!ui.session.state.borrow().is_protected());
}

#[test]
fn the_pin_can_be_changed_and_removed() {
    let ui = encrypted();
    unlock(&ui);
    open_pin_settings(&ui);

    type_into(&ui, "Current PIN", "0000");
    type_into(&ui, "New PIN", "13579");
    type_into(&ui, "Repeat PIN", "13579");
    click(&ui, "Change PIN");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::WrongPin));

    type_into(&ui, "Current PIN", PIN);
    type_into(&ui, "New PIN", "13579");
    type_into(&ui, "Repeat PIN", "13579");
    click(&ui, "Change PIN");
    assert_eq!(ui.session.window.get_status_message(), english(&Message::PinChanged));

    type_into(&ui, "Current PIN", "13579");
    click(&ui, "Remove PIN and encryption");
    assert_eq!(ui.session.window.get_status_message(), english(&Message::PinTurnedOff));
    assert!(!ui.session.window.get_pin_set());
    assert!(!ui.session.state.borrow().is_protected());
}

// ---------------------------------------------------------------------------------------------
// Settings: data locations

#[test]
fn moving_the_database_shows_where_it_went() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("moved");
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 0);
    let moved = destination.join(DATABASE_FILE_NAME);
    assert!(moved.is_file());
    assert!(!ui.fixture.database().exists());
    assert_eq!(ui.session.window.get_database_path(), moved.display().to_string());
    let path = moved.display().to_string();
    assert_eq!(ui.session.window.get_status_message(), english(&Message::DatabaseMoved { path: &path }));
    assert_eq!(ui.dialogs.folder_starts.borrow()[0], ui.fixture.path("data"), "opens where the database is");
}

#[test]
fn regression_a_move_that_cannot_delete_the_original_still_shows_the_new_location() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("moved");
    let _held = common::hold_against_removal(&ui.fixture.database());
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 0);
    let moved = destination.join(DATABASE_FILE_NAME);
    assert!(moved.is_file(), "the app switched to the copy");
    assert_eq!(ui.session.window.get_database_path(), moved.display().to_string());
    assert!(!ui.session.window.get_error_message().is_empty(), "the leftover original is reported");
}

#[test]
fn moving_the_database_onto_another_file_is_refused() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("taken");
    fs::create_dir_all(&destination).unwrap();
    fs::write(destination.join(DATABASE_FILE_NAME), b"someone else's").unwrap();
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 0);
    let error = AppError::AlreadyExists(destination.join(DATABASE_FILE_NAME));
    assert_eq!(ui.session.window.get_error_message(), error_text(&error));
    assert!(ui.fixture.database().is_file(), "the database stays where it was");
}

#[test]
fn cancelled_moves_change_nothing() {
    let ui = seeded();
    open_folder_settings(&ui);
    ui.dialogs.pick_folder_next(None);
    ui.dialogs.pick_folder_next(None);
    click_nth(&ui, "Move…", 0);
    click_nth(&ui, "Move…", 1);
    assert!(ui.fixture.database().is_file());
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn moving_the_logs_shows_where_they_went() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("new-logs");
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 1);
    assert!(destination.join(LOG_FILE_NAME).is_file());
    assert_eq!(ui.session.window.get_log_folder(), destination.display().to_string());
    let path = destination.display().to_string();
    assert_eq!(ui.session.window.get_status_message(), english(&Message::LogsMoved { path: &path }));
}

#[test]
fn moving_the_logs_onto_another_log_file_is_refused() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("taken-logs");
    fs::create_dir_all(&destination).unwrap();
    fs::write(destination.join(LOG_FILE_NAME), b"other log").unwrap();
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 1);
    let error = AppError::AlreadyExists(destination.join(LOG_FILE_NAME));
    assert_eq!(ui.session.window.get_error_message(), error_text(&error));
    assert_eq!(ui.session.window.get_log_folder(), ui.fixture.path("logs").display().to_string());
}

#[test]
fn another_database_can_be_opened_from_settings() {
    let other = Fixture::new();
    let mut state = other.set_up();
    classes::create_class(state.connection().unwrap(), "Other class").unwrap();
    state.shutdown();
    let ui = seeded();
    open_folder_settings(&ui);
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open another database…");
    assert_eq!(ui.session.window.get_database_path(), other.database().display().to_string());
    assert_eq!(gradebook(&ui).get_class_name(), "Other class");
}

#[test]
fn opening_a_missing_file_is_reported() {
    let ui = seeded();
    open_folder_settings(&ui);
    let missing = ui.fixture.path("gone.db");
    ui.dialogs.pick_file_next(Some(&missing));
    click(&ui, "Open another database…");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::NotFound(missing)));
    assert_eq!(gradebook(&ui).get_class_name(), "1A", "the open database stays");
}

#[test]
fn a_cancelled_switch_keeps_the_database() {
    let ui = seeded();
    open_folder_settings(&ui);
    ui.dialogs.pick_file_next(None);
    click(&ui, "Open another database…");
    assert_eq!(ui.session.window.get_database_path(), ui.fixture.database().display().to_string());
}

#[test]
fn switching_to_an_encrypted_database_locks() {
    let other = encrypted_fixture();
    let ui = seeded();
    open_folder_settings(&ui);
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open another database…");
    assert!(ui.session.window.get_locked());
}

// ---------------------------------------------------------------------------------------------
// Settings: backup

/// The settings page, scrolled down to the Backup card.
fn open_backup_settings(ui: &Ui) {
    open_settings(ui);
    find(ui, "Galego").scroll(0.0, -1200.0);
    assert!(shows(ui, "Export everything…"), "scrolled to the Backup card");
}

#[test]
fn export_everything_saves_a_backup_and_says_where() {
    let ui = seeded();
    open_backup_settings(&ui);
    let destination = ui.fixture.path("backups").join("mine.avalio");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    ui.dialogs.save_file_next(Some(&destination));
    click(&ui, "Export everything…");

    let copy = db::open_existing(&destination, None).unwrap();
    assert_eq!(classes::list_classes(&copy).unwrap().len(), 1);
    let path = destination.display().to_string();
    assert_eq!(ui.session.window.get_status_message(), english(&Message::BackupExported { path: &path }));
    let requests = ui.dialogs.save_requests.borrow();
    let (kind, _, name) = &requests[0];
    assert_eq!(*kind, FileKind::Backup);
    assert_eq!(*name, backup::default_file_name(chrono::Local::now().date_naive()));
}

#[test]
fn the_next_backup_opens_where_the_last_one_went() {
    let ui = seeded();
    open_backup_settings(&ui);
    let folder = ui.fixture.path("backups");
    fs::create_dir_all(&folder).unwrap();
    ui.dialogs.save_file_next(Some(&folder.join("first.avalio")));
    click(&ui, "Export everything…");
    ui.dialogs.save_file_next(None);
    click(&ui, "Export everything…");
    assert_eq!(ui.dialogs.save_requests.borrow()[1].1, folder);
}

#[test]
fn a_cancelled_backup_does_nothing() {
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.save_file_next(None);
    click(&ui, "Export everything…");
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(fs::read_dir(ui.fixture.root.path()).unwrap().count(), 3, "only config, data and logs");
}

#[test]
fn a_failed_backup_shows_the_error() {
    let ui = seeded();
    open_backup_settings(&ui);
    // a folder already has the chosen name, so the backup can't be put there
    let taken = ui.fixture.path("taken.avalio");
    fs::create_dir_all(&taken).unwrap();
    ui.dialogs.save_file_next(Some(&taken));
    click(&ui, "Export everything…");
    assert!(ui.session.window.get_error_message().contains("taken.avalio"));
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(taken.is_dir());
}

/// A backup of another database with classes 7A (two pupils) and 7B, encrypted with `pin` if given.
fn backup_file(pin: Option<&str>) -> (Fixture, std::path::PathBuf) {
    let other = Fixture::new();
    let mut state = other.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "7A").unwrap();
        pupils::add_new_pupil(connection, class, "Xoán").unwrap();
        pupils::add_new_pupil(connection, class, "Iria").unwrap();
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

fn backup_global(ui: &Ui) -> avalio_lite::ui::Backup<'_> {
    ui.session.window.global::<avalio_lite::ui::Backup>()
}

fn restore_copy(ui: &Ui) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.restore", ui.fixture.database().display()))
}

fn sidebar_classes(ui: &Ui) -> Vec<String> {
    gradebook(ui).get_classes().iter().map(|class| class.name.to_string()).collect()
}

#[test]
fn restore_asks_once_and_replaces_the_sheet() {
    let (_other, file) = backup_file(None);
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    ui.dialogs.answer_next(true);
    click(&ui, "Restore from a backup…");

    let question =
        english(&Message::ConfirmRestore { file: "avalio-lite-backup-2026-10-28.avalio", classes: 2, pupils: 2 });
    assert_eq!(*ui.dialogs.questions.borrow(), [question]);
    assert_eq!(*ui.dialogs.file_kinds.borrow(), [FileKind::Backup]);
    assert_eq!(sidebar_classes(&ui), ["7A", "7B"]);
    assert_eq!(gradebook(&ui).get_class_name(), "7A", "the first class is selected");
    let status = english(&Message::BackupRestored { file: "avalio-lite-backup-2026-10-28.avalio" });
    assert_eq!(ui.session.window.get_status_message(), status);
    assert!(!restore_copy(&ui).exists());
}

#[test]
fn no_to_the_restore_question_changes_nothing() {
    let (_other, file) = backup_file(None);
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    ui.dialogs.answer_next(false);
    click(&ui, "Restore from a backup…");
    assert_eq!(sidebar_classes(&ui), ["1A"]);
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2);
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(!restore_copy(&ui).exists());
}

#[test]
fn a_cancelled_restore_picker_does_nothing() {
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(None);
    click(&ui, "Restore from a backup…");
    assert!(ui.dialogs.questions.borrow().is_empty());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
}

#[test]
fn a_file_that_is_not_a_backup_shows_the_error() {
    let ui = seeded();
    open_backup_settings(&ui);
    let file = ui.fixture.path("notes.avalio");
    fs::write(&file, b"hello").unwrap();
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::NotADatabase(file)));
    assert!(!backup_global(&ui).get_pin_needed());
    assert!(ui.dialogs.questions.borrow().is_empty());
}

#[test]
fn an_encrypted_backup_asks_for_its_pin_inline() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    assert!(backup_global(&ui).get_pin_needed());
    assert!(shows_text(&ui, "This backup is encrypted. Type the PIN it was made with."));
    assert!(backup_global(&ui).get_pin_error().is_empty(), "no error before a PIN is typed");
    assert!(ui.dialogs.questions.borrow().is_empty(), "no question before the backup opens");

    ui.dialogs.answer_next(true);
    type_into(&ui, "Backup PIN", "9753");
    press_enter(&ui);
    assert!(!backup_global(&ui).get_pin_needed(), "the prompt closes");
    assert_eq!(ui.dialogs.questions.borrow().len(), 1);
    assert_eq!(sidebar_classes(&ui), ["7A", "7B"]);
    assert!(!ui.session.state.borrow().is_protected(), "the app keeps having no PIN");
}

#[test]
fn a_wrong_backup_pin_keeps_the_prompt() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    type_into(&ui, "Backup PIN", "1111");
    click(&ui, "Continue");
    assert!(backup_global(&ui).get_pin_needed());
    assert_eq!(backup_global(&ui).get_pin_error(), error_text(&AppError::WrongPin));
    assert!(backup_global(&ui).get_pin().is_empty(), "the wrong PIN is cleared");
    assert!(!restore_copy(&ui).exists());

    click(&ui, "Cancel");
    assert!(!backup_global(&ui).get_pin_needed());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
}

#[test]
fn leaving_settings_closes_the_backup_prompt() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    assert!(backup_global(&ui).get_pin_needed());
    click(&ui, "1A");
    assert!(!backup_global(&ui).get_pin_needed());
    // nothing is left waiting: a stray Continue does nothing
    backup_global(&ui).invoke_pin_entered();
    assert_eq!(sidebar_classes(&ui), ["1A"]);
}

#[test]
fn locking_closes_the_backup_prompt() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = encrypted();
    unlock(&ui);
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    assert!(backup_global(&ui).get_pin_needed());
    with_control(&ui, "l");
    assert!(ui.session.window.get_locked());
    assert!(!backup_global(&ui).get_pin_needed());
    assert!(!restore_copy(&ui).exists());
}

#[test]
fn a_backup_of_this_database_restores_without_asking_for_its_pin() {
    let ui = encrypted();
    unlock(&ui);
    open_backup_settings(&ui);
    let file = ui.fixture.path("mine.avalio");
    ui.dialogs.save_file_next(Some(&file));
    click(&ui, "Export everything…");
    ui.dialogs.pick_file_next(Some(&file));
    ui.dialogs.answer_next(true);
    click(&ui, "Restore from a backup…");
    assert!(!backup_global(&ui).get_pin_needed());
    assert_eq!(ui.dialogs.questions.borrow().len(), 1);
    assert!(ui.session.state.borrow().is_protected(), "still encrypted with the app's PIN");
}

#[test]
fn a_failed_restore_shows_the_error_and_keeps_the_sheet() {
    let (_other, file) = backup_file(None);
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    ui.dialogs.answer_next(true);
    let held = common::hold_against_removal(&ui.fixture.database());
    click(&ui, "Restore from a backup…");
    drop(held);
    assert!(!ui.session.window.get_error_message().is_empty());
    assert!(ui.session.window.get_status_message().is_empty());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
    assert!(!ui.session.window.get_locked());
}

#[test]
fn add_classes_from_a_backup_lists_them_and_reports() {
    let (other, file) = backup_file(None);
    // the backup also has a 1A, so the one added is renamed; and Ana, who is already here
    {
        let mut state = other.state();
        state.start();
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, class, "Ana").unwrap();
        fs::remove_file(&file).unwrap();
        state.export_backup(&file).unwrap();
        state.shutdown();
    }
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Add classes from a backup…");

    assert!(ui.dialogs.questions.borrow().is_empty(), "nothing is replaced, so nothing is asked");
    assert_eq!(sidebar_classes(&ui), ["1A", "1A (2)", "7A", "7B"]);
    assert_eq!(gradebook(&ui).get_class_name(), "1A (2)", "the first class added is open");
    let added = english(&Message::BackupClassesAdded {
        classes: 3,
        pupils: 3,
        existing: 1,
        file: "avalio-lite-backup-2026-10-28.avalio",
    });
    let renamed = english(&Message::BackupClassesRenamed { pairs: &[("1A".to_owned(), "1A (2)".to_owned())] });
    assert_eq!(ui.session.window.get_status_message(), format!("{added} {renamed}"));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 4, "Ana joins; Xoán and Iria are new");
    assert!(!restore_copy(&ui).exists());
}

#[test]
fn adding_from_an_encrypted_backup_asks_for_its_pin() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Add classes from a backup…");
    assert!(backup_global(&ui).get_pin_needed());
    type_into(&ui, "Backup PIN", "9753");
    click(&ui, "Continue");
    assert!(!backup_global(&ui).get_pin_needed());
    assert_eq!(sidebar_classes(&ui), ["1A", "7A", "7B"]);
    assert!(!ui.session.state.borrow().is_protected(), "the app's own encryption doesn't change");
}

#[test]
fn adding_from_an_empty_backup_says_so() {
    let other = Fixture::new();
    let file = other.path("empty.avalio");
    let mut state = other.set_up();
    state.export_backup(&file).unwrap();
    state.shutdown();
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Add classes from a backup…");
    assert_eq!(ui.session.window.get_status_message(), english(&Message::BackupHasNoClasses));
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
}

#[test]
fn a_failed_add_shows_the_error_and_adds_nothing() {
    let (_other, file) = backup_file(None);
    let ui = seeded();
    open_backup_settings(&ui);
    // the open database refuses new classes from here on
    ui.session
        .state
        .borrow()
        .connection()
        .unwrap()
        .execute_batch("CREATE TEMP TRIGGER refuse BEFORE INSERT ON classes BEGIN SELECT RAISE(ABORT, 'test'); END")
        .unwrap();
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Add classes from a backup…");
    assert!(!ui.session.window.get_error_message().is_empty());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
    assert!(!restore_copy(&ui).exists());
}

// ---------------------------------------------------------------------------------------------
// Settings: language and theme

#[test]
fn choosing_galician_switches_the_interface_and_numbers() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "7.5");
    open_settings(&ui);
    click(&ui, "Galego");
    assert_eq!(ui.session.window.get_language_code(), "gl");
    assert_eq!(saved_preferences(&ui.fixture).language, Some(Language::Galician));
    assert!(shows(&ui, "Escuro"), "the settings page is in Galician");
    assert_eq!(cell_text(&ui, 0, 1), "7,5", "calculated numbers use a decimal comma");
    assert_eq!(cell_text(&ui, 0, 0), "7.5", "typed text stays as typed");
    click(&ui, "English");
    assert_eq!(cell_text(&ui, 0, 1), "7.5");
    assert_eq!(saved_preferences(&ui.fixture).language, Some(Language::English));
}

#[test]
fn the_theme_buttons_and_sidebar_toggle_switch_and_save() {
    let ui = seeded();
    let theme = ui.session.window.global::<Theme>();
    open_settings(&ui);
    click(&ui, "Dark");
    assert!(theme.get_dark());
    assert_eq!(saved_preferences(&ui.fixture).theme, Some(ThemeChoice::Dark));
    click(&ui, "Light");
    assert!(!theme.get_dark());
    assert_eq!(saved_preferences(&ui.fixture).theme, Some(ThemeChoice::Light));
    click(&ui, "Dark mode");
    assert!(theme.get_dark());
    click(&ui, "Light mode");
    assert!(!theme.get_dark());
    assert_eq!(saved_preferences(&ui.fixture).theme, Some(ThemeChoice::Light));
}

#[test]
fn tab_reaches_the_sidebar_buttons_and_enter_or_space_presses_them() {
    let ui = seeded();
    let theme = ui.session.window.global::<Theme>();
    click(&ui, "New class");
    key(&ui, Key::Tab);
    press_enter(&ui);
    assert!(theme.get_dark(), "Tab moved from the field to the theme button and Enter pressed it");
    key(&ui, " ");
    assert!(!theme.get_dark(), "Space pressed it again");
    key(&ui, Key::Tab);
    press_enter(&ui);
    assert!(shows_text(&ui, "Appearance"), "the next Tab stop is Settings");
}

#[test]
fn screen_readers_can_press_the_custom_controls() {
    let ui = seeded();
    find(&ui, "Settings").invoke_accessible_default_action();
    assert!(shows_text(&ui, "Appearance"));
    find(&ui, "Dark").invoke_accessible_default_action();
    assert!(ui.session.window.global::<Theme>().get_dark());
}

#[test]
fn screen_readers_can_press_the_lock_screen_keys() {
    let ui = encrypted();
    find(&ui, "Letters").invoke_accessible_default_action();
    find(&ui, "q").invoke_accessible_default_action();
    find(&ui, "Numbers").invoke_accessible_default_action();
    find(&ui, "1").invoke_accessible_default_action();
    assert_eq!(ui.session.window.get_unlock_pin(), "q1");
    find(&ui, "Erase").invoke_accessible_default_action();
    find(&ui, "Erase").invoke_accessible_default_action();
    for digit in PIN.chars() {
        find(&ui, &digit.to_string()).invoke_accessible_default_action();
    }
    find(&ui, "Unlock").invoke_accessible_default_action();
    assert!(!ui.session.window.get_locked());
}

#[test]
fn preferences_that_cannot_be_saved_are_reported() {
    let ui = seeded();
    open_settings(&ui);
    // the settings folder turns into a file, so config.toml can't be written
    let config_dir = ui.fixture.path("config");
    fs::remove_dir_all(&config_dir).unwrap();
    fs::write(&config_dir, b"in the way").unwrap();
    click(&ui, "Dark");
    assert!(!ui.session.window.get_error_message().is_empty(), "theme");
    click(&ui, "Galego");
    assert!(!ui.session.window.get_error_message().is_empty(), "language");
    assert_eq!(ui.session.window.get_language_code(), "en", "the language stays when it can't be saved");
    click(&ui, "1A");
    click(&ui, "More actions");
    click(&ui, "Hide weights & scales");
    assert!(gradebook(&ui).get_show_column_settings(), "the column settings stay shown");
    assert!(!ui.session.window.get_error_message().is_empty());
}

// ---------------------------------------------------------------------------------------------
// Classes

#[test]
fn a_class_is_created_from_the_empty_page() {
    let ui = Ui::with_database();
    assert!(shows_text(&ui, "Start with a class"));
    type_into(&ui, "Class name, e.g. 2º ESO A", "2º ESO A");
    click(&ui, "Create class");
    assert_eq!(gradebook(&ui).get_class_name(), "2º ESO A");
    assert!(shows(&ui, "2º ESO A"), "listed in the sidebar");
}

#[test]
fn an_empty_class_name_is_refused() {
    let ui = Ui::with_database();
    click(&ui, "Create class");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::EmptyName));
    assert_eq!(gradebook(&ui).get_classes().row_count(), 0);
}

#[test]
fn a_class_is_created_from_the_sidebar_and_selected() {
    let ui = seeded();
    type_into(&ui, "New class", "3C");
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_class_name(), "3C");
    assert_eq!(gradebook(&ui).get_classes().row_count(), 2);
}

#[test]
fn a_duplicate_class_name_is_refused() {
    let ui = seeded();
    type_into(&ui, "New class", "1A");
    press_enter(&ui);
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::DuplicateClassName("1A".into())));
}

#[test]
fn clicking_a_class_opens_its_sheet() {
    let ui = with_second_class();
    click(&ui, "2B");
    assert_eq!(gradebook(&ui).get_class_name(), "2B");
    assert_eq!(gradebook(&ui).get_rows().row_data(0).unwrap().name, "Carla");
}

#[test]
fn the_class_is_renamed_in_place() {
    let ui = seeded();
    type_into(&ui, "Class name", "1A bis");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM classes WHERE name = '1A bis'"), 1);
    assert!(shows(&ui, "1A bis"), "the sidebar follows");
}

#[test]
fn deleting_a_class_asks_first() {
    let ui = with_second_class();
    click(&ui, "More actions");
    ui.dialogs.answer_next(false);
    click(&ui, "Delete class");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM classes"), 2, "no keeps it");
    assert_eq!(ui.dialogs.questions.borrow()[0], english(&Message::ConfirmDeleteClass { class: "1A" }));

    click(&ui, "More actions");
    ui.dialogs.answer_next(true);
    click(&ui, "Delete class");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM classes"), 1);
    assert_eq!(gradebook(&ui).get_class_name(), "2B", "the next class opens");
}

// ---------------------------------------------------------------------------------------------
// Excel

fn export_through_the_menu(ui: &Ui, destination: Option<&std::path::Path>) {
    click(ui, "More actions");
    ui.dialogs.save_file_next(destination);
    click(ui, "Export to Excel…");
}

#[test]
fn export_to_excel_saves_the_sheet_and_says_where() {
    let ui = seeded();
    let destination = ui.fixture.path("1A.xlsx");
    export_through_the_menu(&ui, Some(&destination));
    assert!(destination.is_file());
    let shown = destination.display().to_string();
    assert_eq!(ui.session.window.get_status_message(), english(&Message::SheetExported { class: "1A", path: &shown }));
    assert!(ui.session.window.get_error_message().is_empty());
    let documents = dirs::document_dir().unwrap_or_default();
    assert_eq!(
        ui.dialogs.save_requests.borrow()[0],
        (FileKind::Spreadsheet, documents, "1A.xlsx".to_owned()),
        "Documents, the first time"
    );

    export_through_the_menu(&ui, Some(&ui.fixture.path("again.xlsx")));
    assert_eq!(ui.dialogs.save_requests.borrow()[1].1, ui.fixture.root.path(), "then where the last one went");
}

#[test]
fn the_proposed_file_name_leaves_out_what_windows_refuses() {
    let ui = seeded();
    type_into(&ui, "Class name", "1A/1B: maths");
    export_through_the_menu(&ui, None);
    assert_eq!(ui.dialogs.save_requests.borrow()[0].2, "1A-1B- maths.xlsx");
}

#[test]
fn a_cancelled_export_does_nothing() {
    let ui = seeded();
    export_through_the_menu(&ui, None);
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(
        fs::read_dir(ui.fixture.root.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "xlsx"))
            .count(),
        0
    );
}

#[test]
fn a_failed_export_shows_the_error() {
    let ui = seeded();
    let destination = ui.fixture.path("no such folder").join("1A.xlsx");
    export_through_the_menu(&ui, Some(&destination));
    let banner = ui.session.window.get_error_message().to_string();
    assert!(banner.starts_with(&format!("Couldn't save {}. ", destination.display())), "{banner}");
    assert!(ui.session.window.get_status_message().is_empty());

    export_through_the_menu(&ui, Some(&ui.fixture.path("1A.xlsx")));
    assert!(ui.session.window.get_error_message().is_empty(), "a later export that works clears it");
    assert!(!ui.session.window.get_status_message().is_empty());
    assert_eq!(
        ui.dialogs.save_requests.borrow()[1].1,
        dirs::document_dir().unwrap_or_default(),
        "a failed export's folder isn't remembered"
    );
}

/// Was: nothing cleared the green banner, so "Saved 1A to …" stayed next to later work.
#[test]
fn regression_the_export_status_clears_on_the_next_action() {
    let ui = seeded();
    export_through_the_menu(&ui, Some(&ui.fixture.path("1A.xlsx")));
    assert!(!ui.session.window.get_status_message().is_empty());
    type_into(&ui, "Class name", "1A bis");
    assert!(ui.session.window.get_status_message().is_empty(), "a sheet action clears it");

    export_through_the_menu(&ui, Some(&ui.fixture.path("1A.xlsx")));
    click(&ui, "More actions");
    click(&ui, "Hide weights & scales");
    assert!(ui.session.window.get_status_message().is_empty(), "so does a preference");
}

/// A workbook in the fixture folder whose first tab holds `rows`, every cell written as text
/// except numbers, which are written as numbers.
fn import_file(ui: &Ui, rows: &[&[&str]]) -> std::path::PathBuf {
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

fn import_through_the_menu(ui: &Ui, file: Option<&std::path::Path>) {
    click(ui, "More actions");
    ui.dialogs.pick_file_next(file);
    click(ui, "Import from Excel…");
}

#[test]
fn import_from_excel_fills_the_sheet_and_reports() {
    let ui = seeded();
    let file = import_file(
        &ui,
        &[&["Pupil", "Exam", "Phone", "", "Final grade"], &["Ana", "7", "600", "", "9"], &["Carla", "8", "", "x", ""]],
    );
    import_through_the_menu(&ui, Some(&file));
    assert_eq!(*ui.dialogs.file_kinds.borrow(), [FileKind::Spreadsheet]);
    assert!(ui.session.window.get_error_message().is_empty(), "{}", ui.session.window.get_error_message());
    assert_eq!(
        ui.session.window.get_status_message(),
        "Imported 2 pupils (1 new) and 2 marks or notes into 1A. Left out: Phone, column D (no title), Final grade."
    );
    assert!(ui.dialogs.questions.borrow().is_empty(), "nothing to replace, so nothing to ask");
    assert_eq!(gradebook(&ui).get_rows().row_count(), 3, "Carla joins the sheet");
    assert_eq!(cell_text(&ui, 0, 0), "7");
    assert_eq!(cell_text(&ui, 2, 0), "8");
    assert_eq!(gradebook(&ui).get_classes().row_data(0).unwrap().pupil_count, 3, "the class list counts her too");
}

#[test]
fn import_reports_skipped_error_cells_and_singulars() {
    let ui = seeded();
    let file = import_file(&ui, &[&["Pupil", "Exam"], &["Ana", "#DIV/0!"]]);
    import_through_the_menu(&ui, Some(&file));
    assert_eq!(
        ui.session.window.get_status_message(),
        "Imported 1 pupil and 0 marks or notes into 1A. 1 cell with an Excel error was skipped."
    );
}

#[test]
fn overwriting_asks_first_and_no_changes_nothing() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "3");
    let file = import_file(&ui, &[&["Pupil", "Exam"], &["Ana", "9"], &["Bea", "6"]]);
    ui.dialogs.answer_next(false);
    import_through_the_menu(&ui, Some(&file));
    assert_eq!(
        *ui.dialogs.questions.borrow(),
        [english(&Message::ConfirmImportOverwrite { count: 1, class: "1A" })],
        "only Ana's 3 is replaced; Bea's empty cell isn't"
    );
    assert_eq!(cell_text(&ui, 0, 0), "3");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 1, "no change at all");
    assert!(ui.session.window.get_status_message().is_empty());

    ui.dialogs.answer_next(true);
    import_through_the_menu(&ui, Some(&file));
    assert_eq!((cell_text(&ui, 0, 0), cell_text(&ui, 1, 0)), ("9".to_owned(), "6".to_owned()));
    assert_eq!(ui.session.window.get_status_message(), "Imported 2 pupils and 2 marks or notes into 1A.");
}

#[test]
fn a_cancelled_import_does_nothing() {
    let ui = seeded();
    import_through_the_menu(&ui, None);
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2);
}

#[test]
fn an_unreadable_file_shows_the_error() {
    let ui = seeded();
    let file = ui.fixture.path("notes.xlsx");
    fs::write(&file, "not a workbook").unwrap();
    import_through_the_menu(&ui, Some(&file));
    let banner = ui.session.window.get_error_message().to_string();
    assert!(banner.starts_with(&format!("{} isn't a workbook that can be read.", file.display())), "{banner}");
    assert!(ui.session.window.get_status_message().is_empty());

    let header_only = import_file(&ui, &[&["Pupil", "Exam"]]);
    import_through_the_menu(&ui, Some(&header_only));
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::NoPupilsInFile(header_only)));

    let twice = import_file(&ui, &[&["Pupil"], &["Eva"], &["eva"]]);
    import_through_the_menu(&ui, Some(&twice));
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::DuplicatePupilInFile("eva".into())));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "nothing was added");
}

// ---------------------------------------------------------------------------------------------
// Pupils

#[test]
fn add_pupil_focuses_the_new_pupil_field() {
    let ui = seeded();
    click(&ui, "+ Add pupil");
    type_text(&ui, "Carmen");
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_rows().row_count(), 3);
    assert!(gradebook(&ui).get_rows().iter().any(|row| row.name == "Carmen"));
}

#[test]
fn a_pupil_is_renamed_in_place() {
    let ui = seeded();
    click_nth(&ui, "Pupil", 0);
    replace_text(&ui, "Ana María");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils WHERE name = 'Ana María'"), 1);
}

/// Was: the row kept the old name, so the removal question asked about "Ana".
#[test]
fn regression_a_renamed_pupil_is_asked_about_by_the_new_name() {
    let ui = seeded();
    click_nth(&ui, "Pupil", 0);
    replace_text(&ui, "Ana María");
    assert_eq!(gradebook(&ui).get_rows().row_data(0).unwrap().name, "Ana María");
    ui.dialogs.answer_next(false);
    click_nth(&ui, "Remove from this class", 0);
    let question = english(&Message::ConfirmRemovePupil { pupil: "Ana María", class: "1A" });
    assert_eq!(ui.dialogs.questions.borrow()[0], question);
}

/// Was: the delete question still named the class by its old name.
#[test]
fn regression_a_renamed_class_is_asked_about_by_the_new_name() {
    let ui = seeded();
    type_into(&ui, "Class name", "1A bis ");
    assert_eq!(gradebook(&ui).get_class_name(), "1A bis", "saved and shown trimmed");
    click(&ui, "More actions");
    ui.dialogs.answer_next(false);
    click(&ui, "Delete class");
    assert_eq!(ui.dialogs.questions.borrow()[0], english(&Message::ConfirmDeleteClass { class: "1A bis" }));
}

#[test]
fn removing_a_pupil_asks_first() {
    let ui = seeded();
    ui.dialogs.answer_next(false);
    click_nth(&ui, "Remove from this class", 0);
    assert_eq!(gradebook(&ui).get_rows().row_count(), 2);
    let question = english(&Message::ConfirmRemovePupil { pupil: "Ana", class: "1A" });
    assert_eq!(ui.dialogs.questions.borrow()[0], question);

    ui.dialogs.answer_next(true);
    click_nth(&ui, "Remove from this class", 0);
    assert_eq!(gradebook(&ui).get_rows().row_count(), 1);
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "the pupil itself stays");
}

#[test]
fn a_pupil_from_another_class_is_added_from_the_menu() {
    let ui = with_second_class();
    click(&ui, "More actions");
    click(&ui, "Add from another class…");
    click(&ui, "Carla");
    let names: Vec<String> = gradebook(&ui).get_rows().iter().map(|row| row.name.to_string()).collect();
    assert_eq!(names, ["Ana", "Carla"]);
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "the same pupil, not a copy");
}

// ---------------------------------------------------------------------------------------------
// Columns

#[test]
fn every_column_type_can_be_added_from_the_menu() {
    let ui = seeded();
    for (option, title) in [("Info", "Notes"), ("Average", "Average"), ("Rounded", "Rounded"), ("Test", "Test 2")] {
        click(&ui, "+ Add column");
        click_menu_option(&ui, option);
        assert!(gradebook(&ui).get_columns().iter().any(|column| column.title == title), "{title}");
    }
    click(&ui, "+ Add column");
    click_menu_option(&ui, "Final grade");
    assert_eq!(gradebook(&ui).get_columns().row_count(), 7);
}

#[test]
fn clicking_a_header_opens_its_settings_panel() {
    let ui = seeded();
    click(&ui, "Exam");
    assert_eq!(gradebook(&ui).get_panel_index(), 0);
    assert!(shows(&ui, "Weight"));
    click(&ui, "Close");
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert!(!shows(&ui, "Weight"));
}

#[test]
fn the_panel_edits_a_tests_settings() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Title", "Exam 1");
    type_into(&ui, "Weight", "60");
    type_into(&ui, "Out of", "5");
    click(&ui, "Bonus test");
    let exam = gradebook(&ui).get_columns().row_data(0).unwrap();
    assert_eq!(exam.title, "Exam 1");
    assert_eq!(exam.weight, "60");
    assert_eq!(exam.max_score, "5");
    assert!(exam.bonus);
    assert!(!gradebook(&ui).get_weights_complete(), "a bonus test leaves the normal tests at 0%");
}

#[test]
fn an_invalid_weight_is_reported_in_the_banner() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "lots");
    assert!(!ui.session.window.get_error_message().is_empty());
    assert_eq!(gradebook(&ui).get_columns().row_data(0).unwrap().weight, "100", "the weight is unchanged");
}

#[test]
fn weights_that_dont_add_up_warn_and_grey_the_final_grade() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    click(&ui, "Exam");
    type_into(&ui, "Weight", "50");
    assert!(!gradebook(&ui).get_weights_complete());
    assert!(shows_text(&ui, "Test weights add up to 50%, not 100%"));
    let final_grade = gradebook(&ui).get_rows().row_data(0).unwrap().cells.row_data(1).unwrap();
    assert!(final_grade.provisional);
}

#[test]
fn columns_move_and_the_ends_are_disabled() {
    let ui = seeded();
    click(&ui, "Exam");
    assert_eq!(find(&ui, "‹ Move left").accessible_enabled(), Some(false));
    click(&ui, "Move right ›");
    let titles: Vec<String> = gradebook(&ui).get_columns().iter().map(|column| column.title.to_string()).collect();
    assert_eq!(titles, ["Final grade", "Exam"]);
    assert_eq!(gradebook(&ui).get_panel_index(), 1, "the panel follows the column");
    click(&ui, "‹ Move left");
    assert_eq!(gradebook(&ui).get_columns().row_data(0).unwrap().title, "Exam");
}

#[test]
fn deleting_a_column_asks_first() {
    let ui = seeded();
    click(&ui, "Exam");
    ui.dialogs.answer_next(false);
    click(&ui, "Delete");
    assert_eq!(gradebook(&ui).get_columns().row_count(), 2);
    assert_eq!(ui.dialogs.questions.borrow()[0], english(&Message::ConfirmDeleteColumn { column: "Exam" }));
    ui.dialogs.answer_next(true);
    click(&ui, "Delete");
    assert_eq!(gradebook(&ui).get_columns().row_count(), 1);
    assert_eq!(gradebook(&ui).get_panel_index(), -1, "the panel closes with its column");
}

#[test]
fn a_rounded_column_rounds_its_source() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "7.46");
    click(&ui, "+ Add column");
    click_menu_option(&ui, "Rounded");
    let rounded = gradebook(&ui).get_columns().row_data(2).unwrap();
    assert_eq!(rounded.source_index, 1, "rounds the last numeric column, the final grade");
    click(&ui, "Rounded");
    type_into(&ui, "Decimals", "1");
    assert_eq!(cell_text(&ui, 0, 2), "7.5");
    // the source picker's indexes refer to the numeric columns: 0 is the test
    gradebook(&ui).invoke_set_round_source(rounded.id, 0);
    assert_eq!(gradebook(&ui).get_columns().row_data(2).unwrap().source_index, 0);
    assert_eq!(cell_text(&ui, 0, 2), "7.5");
}

#[test]
fn hiding_weights_and_scales_is_saved() {
    let ui = seeded();
    click(&ui, "More actions");
    click(&ui, "Hide weights & scales");
    assert!(!gradebook(&ui).get_show_column_settings());
    assert!(!saved_preferences(&ui.fixture).show_column_settings);
    click(&ui, "More actions");
    click(&ui, "Show weights & scales");
    assert!(gradebook(&ui).get_show_column_settings());
}

// ---------------------------------------------------------------------------------------------
// Cells and summary

/// When the file changed under the open sheet (a pupil or a column added from elsewhere), an
/// in-place refresh would put values in the wrong rows or columns, so the whole sheet is redrawn.
#[test]
fn a_sheet_that_changed_underneath_is_redrawn_whole() {
    let ui = seeded();
    {
        let state = ui.session.state.borrow();
        let connection = state.connection().unwrap();
        let class = classes::list_classes(connection).unwrap()[0].id;
        pupils::add_new_pupil(connection, class, "Carla").unwrap();
    }
    type_into(&ui, "Ana, Exam", "8");
    assert_eq!(gradebook(&ui).get_rows().row_count(), 3, "the new pupil shows");
    {
        let state = ui.session.state.borrow();
        let connection = state.connection().unwrap();
        let class = classes::list_classes(connection).unwrap()[0].id;
        columns::add_column(connection, class, ColumnType::Info, Language::English).unwrap();
    }
    let exam = gradebook(&ui).get_columns().row_data(0).unwrap().id;
    gradebook(&ui).invoke_set_weight(exam, "100".into());
    assert_eq!(gradebook(&ui).get_columns().row_count(), 3, "the new column shows");
    assert_eq!(cell_text(&ui, 0, 0), "8", "and the marks stay in their places");
}

#[test]
fn typing_marks_updates_the_final_grade_and_summary() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    type_into(&ui, "Bea, Exam", "3");
    assert_eq!(cell_text(&ui, 0, 1), "8");
    let rows = gradebook(&ui).get_rows();
    assert_eq!(rows.row_data(0).unwrap().cells.row_data(1).unwrap().tone, Tone::Pass);
    assert_eq!(rows.row_data(1).unwrap().cells.row_data(1).unwrap().tone, Tone::Fail);
    assert!(gradebook(&ui).get_has_summary());
    assert_eq!(gradebook(&ui).get_class_average(), "5.5");
    assert_eq!((gradebook(&ui).get_passing(), gradebook(&ui).get_failing()), (1, 1));
    assert!(shows_text(&ui, "Class average 5.5"));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 2, "typed marks are saved");
}

#[test]
fn an_out_of_range_mark_is_flagged_and_ignored() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "12");
    let cell = gradebook(&ui).get_rows().row_data(0).unwrap().cells.row_data(0).unwrap();
    assert!(cell.invalid);
    assert_eq!(cell_text(&ui, 0, 1), "", "the final grade leaves it out, so nothing is marked yet");
}

#[test]
fn clearing_a_mark_deletes_it() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    type_into(&ui, "Ana, Exam", "");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 0);
    assert!(!gradebook(&ui).get_has_summary());
}

// ---------------------------------------------------------------------------------------------
// Actions without an open database

#[test]
fn sheet_actions_while_locked_report_no_database() {
    let ui = encrypted();
    gradebook(&ui).invoke_create_class("1A".into());
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::NoDatabaseOpen));
}

// ---------------------------------------------------------------------------------------------
// Regressions: each test failed before its fix. Run them alone with `cargo test regression`.

/// Ctrl+L only reaches the shortcut handler while something in the window has focus. After the
/// PIN field that unlocked the app went away, nothing had focus and Ctrl+L did nothing.
#[test]
fn regression_ctrl_l_locks_right_after_unlocking() {
    let ui = encrypted();
    unlock(&ui);
    with_control(&ui, "l");
    assert!(ui.session.window.get_locked());
}

/// Same cause: a database without a PIN opened straight onto the sheet with nothing focused,
/// so Ctrl+L didn't even show the hint about setting a PIN.
#[test]
fn regression_ctrl_l_answers_right_after_startup() {
    let ui = seeded();
    with_control(&ui, "l");
    assert_eq!(ui.session.window.get_error_message(), english(&Message::LockNeedsPin));
}

/// Same cause: typing in a settings field and then going back to the sheet destroyed the
/// focused field, and Ctrl+L stopped working until something else was clicked.
#[test]
fn regression_ctrl_l_locks_after_leaving_a_focused_settings_field() {
    let fixture = encrypted_fixture();
    let ui = Ui::open(fixture);
    unlock(&ui);
    type_into(&ui, "New class", "1A");
    press_enter(&ui);
    open_pin_settings(&ui);
    type_into(&ui, "Current PIN", "1");
    click(&ui, "1A");
    with_control(&ui, "l");
    assert!(ui.session.window.get_locked());
}

/// In Galician at the window's minimum width, the sheet's title row (class name, summary line and
/// the three buttons) was wider than the window: "+ Engadir columna" was cut off and "⋯" (with
/// "Delete class" behind it) was pushed out of sight. Found by the desktop tests' Galician check.
#[test]
fn regression_sheet_actions_fit_the_smallest_window_in_galician() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    common::sample_class(state.connection().unwrap());
    state.set_language(Language::Galician).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    ui.session.window.window().set_size(slint::LogicalSize::new(1000.0, 600.0));
    for english in ["+ Add pupil", "+ Add column", "More actions"] {
        let label = common::galician(english);
        let control = find(&ui, &label);
        let right = control.absolute_position().x + control.size().width;
        assert!(right <= 1000.0, "{label} ends at {right}, past the 1000 px window");
    }
}

/// Menu options had a fixed height: a hint that wrapped to two lines (most of the add-column
/// menu) overflowed it, so the title touched the menu's top edge and the last hint was cut off.
/// Found by the desktop tests' screenshots.
#[test]
fn regression_menu_options_are_tall_enough_for_their_hints() {
    let ui = seeded();
    click(&ui, "+ Add column");
    for (title, hint) in [
        ("Test", "A mark out of a maximum that counts towards the final grade"),
        ("Rounded", "Another column rounded to whole marks or decimals"),
    ] {
        let option = elements(&ui, title)
            .into_iter()
            .find(|element| element.accessible_description().is_some_and(|text| text == hint))
            .unwrap();
        let text = ElementHandle::find_by_accessible_label(&ui.session.window, hint).next().unwrap();
        let (top, bottom) = (option.absolute_position().y, option.absolute_position().y + option.size().height);
        let text_bottom = text.absolute_position().y + text.size().height;
        assert!(text.size().height > 20.0, "the {title} hint wraps to two lines in the 340 px menu");
        assert!(text_bottom <= bottom, "the {title} hint ends at {text_bottom}, below the option's bottom {bottom}");
        assert!(text.absolute_position().y > top, "inside the option");
    }
}

/// In Galician the column panel's three buttons ("‹ Mover á esquerda", "Mover á dereita ›",
/// "Eliminar") were wider than the panel, and "Eliminar" was pushed out of the window entirely,
/// so a column couldn't be deleted. Found by the desktop tests' Galician check.
#[test]
fn regression_column_panel_buttons_fit_in_galician() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    common::sample_class(state.connection().unwrap());
    state.set_language(Language::Galician).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    ui.session.window.window().set_size(slint::LogicalSize::new(1000.0, 600.0));
    click(&ui, "Exam 1");
    let close = find(&ui, &common::galician("Close"));
    let panel_right = close.absolute_position().x + close.size().width + 24.0;
    for english in ["‹ Move left", "Move right ›", "Delete"] {
        let label = common::galician(english);
        let control = find(&ui, &label);
        let right = control.absolute_position().x + control.size().width;
        assert!(right <= panel_right.min(1000.0), "{label} ends at {right}, past the panel ({panel_right})");
    }
}

/// Number fields in the column panel sat in a box that asked the layout for no height, so once
/// the panel stopped stretching its settings the fields covered their own labels ("Weight",
/// "Out of"). Found reviewing the desktop baselines.
#[test]
fn regression_number_fields_sit_below_their_labels() {
    let ui = seeded();
    click(&ui, "Exam");
    for label in ["Weight", "Out of"] {
        let field = find(&ui, label);
        let text = ElementHandle::find_by_accessible_label(&ui.session.window, label)
            .find(|element| element.type_name().as_deref() == Some("FieldLabel"))
            .unwrap();
        let text_bottom = text.absolute_position().y + text.size().height;
        assert!(
            text_bottom <= field.absolute_position().y,
            "{label}: the label ends at {text_bottom}, below the field's top {}",
            field.absolute_position().y
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Messages on the page that is open, menus and fields from the keyboard

/// The settings folder turns into a file, so nothing the app saves to config.toml can be written.
fn break_saving_the_settings(ui: &Ui) {
    let config_dir = ui.fixture.path("config");
    fs::remove_dir_all(&config_dir).unwrap();
    fs::write(&config_dir, b"in the way").unwrap();
}

/// What the field labelled `label` holds.
fn field_text(ui: &Ui, label: &str) -> String {
    find(ui, label).accessible_value().map(|value| value.to_string()).unwrap_or_default()
}

/// The error on the window, which must also be on screen.
fn error_on_screen(ui: &Ui) -> String {
    let message = ui.session.window.get_error_message().to_string();
    assert!(!message.is_empty(), "there is an error");
    assert!(shows_text(ui, &message), "the error is shown on the open page: {message}");
    message
}

/// The sheet showed `Gradebook.error-message` and Settings the window's, so a failure reported
/// from the other page (the theme button in the sidebar, Ctrl+L) was set but never seen.
#[test]
fn regression_a_theme_failure_from_the_sheet_shows_on_the_sheet() {
    let ui = seeded();
    break_saving_the_settings(&ui);
    click(&ui, "Dark mode");
    error_on_screen(&ui);
}

#[test]
fn regression_the_lock_hint_shows_on_the_sheet_and_on_settings() {
    let ui = seeded();
    with_control(&ui, "l");
    assert_eq!(error_on_screen(&ui), english(&Message::LockNeedsPin));
    open_settings(&ui);
    assert!(ui.session.window.get_error_message().is_empty(), "leaving the sheet drops its message");
    with_control(&ui, "l");
    assert_eq!(error_on_screen(&ui), english(&Message::LockNeedsPin));
}

#[test]
fn regression_messages_are_dropped_when_the_page_changes() {
    let ui = seeded();
    break_saving_the_settings(&ui);
    click(&ui, "Dark mode");
    error_on_screen(&ui);
    open_settings(&ui);
    assert!(ui.session.window.get_error_message().is_empty());
    click(&ui, "Dark");
    error_on_screen(&ui);
    click(&ui, "1A");
    assert!(ui.session.window.get_error_message().is_empty());
    assert!(ui.session.window.get_status_message().is_empty());
}

#[test]
fn regression_ignored_shortcut_entries_show_on_screen() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    with_shortcut_config(&fixture, &[("lock", "Ctrl+")]);
    let ui = Ui::open(fixture);
    assert!(error_on_screen(&ui).contains("lock = \"Ctrl+\""));
}

/// The panel is matched to its column by row id, and a new database has columns with the same ids.
#[test]
fn regression_opening_another_database_closes_the_column_panel() {
    let other = Fixture::new();
    let mut state = other.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "Other class").unwrap();
        columns::add_column(connection, class, ColumnType::Test, Language::English).unwrap();
    }
    state.shutdown();
    let ui = seeded();
    click(&ui, "Exam");
    assert_eq!(gradebook(&ui).get_panel_index(), 0);
    open_folder_settings(&ui);
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open another database…");
    assert_eq!(gradebook(&ui).get_class_name(), "Other class");
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert_eq!(gradebook(&ui).get_panel_column().row_count(), 0);
}

#[test]
fn regression_deleting_the_last_class_closes_the_column_panel() {
    let ui = seeded();
    click(&ui, "Exam");
    ui.dialogs.answer_next(true);
    gradebook(&ui).invoke_delete_class();
    assert_eq!(gradebook(&ui).get_current_class_id(), -1);
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert_eq!(gradebook(&ui).get_panel_column().row_count(), 0);
}

/// A calculated cell's value sat in a plain Text next to a hidden field, so a screen reader had
/// no cell to read it from.
#[test]
fn regression_calculated_cells_are_readable_with_their_value() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    let value = cell_text(&ui, 0, 1);
    assert!(!value.is_empty());
    assert!(shows_text(&ui, &format!("Ana, Final grade: {value}")));
    assert!(!shows_text(&ui, &format!("Ana, Final grade: {value} (provisional)")));
    click(&ui, "Exam");
    type_into(&ui, "Weight", "50");
    let value = cell_text(&ui, 0, 1);
    assert!(shows_text(&ui, &format!("Ana, Final grade: {value} (provisional)")));
    assert!(shows_text(&ui, "Bea, Final grade"), "an empty calculated cell is still named");
}

#[test]
fn regression_the_add_column_menu_works_from_the_keyboard() {
    let ui = seeded();
    click(&ui, "+ Add column");
    key(&ui, Key::DownArrow);
    press_enter(&ui);
    let columns = gradebook(&ui).get_columns();
    assert_eq!(columns.row_count(), 3);
    assert_eq!(columns.row_data(2).unwrap().kind, avalio_lite::ui::ColumnType::Info, "Down moved to Info");
    assert!(!shows(&ui, "Rounded"), "choosing closes the menu");

    click(&ui, "+ Add column");
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_columns().row_count(), 4, "the first option is the one on");
    assert_eq!(gradebook(&ui).get_columns().row_data(3).unwrap().kind, avalio_lite::ui::ColumnType::Test);

    click(&ui, "+ Add column");
    key(&ui, Key::UpArrow);
    key(&ui, Key::Escape);
    assert!(!shows(&ui, "Rounded"), "Escape closes the menu");
    assert_eq!(gradebook(&ui).get_columns().row_count(), 4, "and adds nothing");
}

#[test]
fn regression_the_more_menu_works_from_the_keyboard() {
    let ui = seeded();
    click(&ui, "More actions");
    press_enter(&ui);
    assert!(!gradebook(&ui).get_show_column_settings(), "the first option toggles the weights and scales");
    click(&ui, "More actions");
    key(&ui, Key::UpArrow);
    key(&ui, Key::Escape);
    assert!(!shows(&ui, "Delete class"), "Escape closes the menu");
    assert_eq!(gradebook(&ui).get_classes().row_count(), 1);
}

#[test]
fn regression_the_column_panel_starts_on_its_title_and_escape_closes_it() {
    let ui = seeded();
    click(&ui, "Exam");
    replace_text(&ui, "Quiz");
    assert_eq!(gradebook(&ui).get_columns().row_data(0).unwrap().title, "Quiz", "typing went to the title");
    key(&ui, Key::Escape);
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert!(!shows(&ui, "Weight"));
}

#[test]
fn regression_closing_the_panel_takes_a_refused_value_error_with_it() {
    // Escape on a field holding a refused weight closed the panel but left its error showing
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "x");
    assert!(!ui.session.window.get_error_message().is_empty(), "the refused weight is reported");
    key(&ui, Key::Escape);
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert!(ui.session.window.get_error_message().is_empty(), "{}", ui.session.window.get_error_message());
}

#[test]
fn a_stale_column_index_opens_no_panel() {
    // a click that lands after the column went (a reload in between) must not open anything
    let ui = seeded();
    gradebook(&ui).invoke_open_column_panel(99);
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn regression_enter_in_the_repeat_pin_field_sets_the_pin() {
    let ui = seeded();
    open_pin_settings(&ui);
    type_into(&ui, "PIN", PIN);
    type_into(&ui, "Repeat PIN", PIN);
    press_enter(&ui);
    assert!(ui.session.window.get_pin_set(), "{}", ui.session.window.get_error_message());
    assert_eq!(ui.session.window.get_status_message(), english(&Message::PinTurnedOn));
}

/// Saving as you type leaves a refused value (empty, a taken name) in the field; the field kept
/// it when left, out of step with what was saved.
#[test]
fn regression_a_refused_class_name_is_put_back_when_the_field_is_left() {
    let ui = with_second_class();
    click(&ui, "Class name");
    replace_text(&ui, "2B");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::DuplicateClassName("2B".into())));
    assert_eq!(field_text(&ui, "Class name"), "2B", "the refused text stays while typing");
    click(&ui, "Add pupil…");
    // "2" was saved on the way to "2B" (saving as you type), and that is what the field goes back to
    assert_eq!(field_text(&ui, "Class name"), "2");
    assert_eq!(gradebook(&ui).get_class_name(), "2");
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn regression_enter_puts_back_a_refused_pupil_name() {
    let ui = seeded();
    let ana = || elements(&ui, "Pupil").into_iter().find(|e| e.accessible_value().as_deref() == Some("Ana"));
    click_element(&ana().unwrap());
    replace_text(&ui, "");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::EmptyName));
    press_enter(&ui);
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(gradebook(&ui).get_rows().row_data(0).unwrap().name, "Ana");
    assert!(ana().is_some(), "the field shows the saved name again");
}

#[test]
fn regression_a_refused_column_value_is_put_back_when_the_panel_field_is_left() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "lots");
    error_on_screen(&ui);
    assert_eq!(field_text(&ui, "Weight"), "lots");
    type_into(&ui, "Out of", "10");
    assert_eq!(field_text(&ui, "Weight"), "100");
    assert!(ui.session.window.get_error_message().is_empty());
    type_into(&ui, "Title", "");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::EmptyName));
    press_enter(&ui);
    assert_eq!(field_text(&ui, "Title"), "Exam");
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn a_value_that_was_saved_is_kept_when_the_field_is_left() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Title", "Quiz");
    type_into(&ui, "Weight", "60");
    assert_eq!(field_text(&ui, "Title"), "Quiz");
    assert_eq!(field_text(&ui, "Weight"), "60");
}

#[test]
fn regression_a_refused_new_class_or_pupil_stays_in_its_field() {
    let ui = seeded();
    type_into(&ui, "New class", "1A");
    press_enter(&ui);
    error_on_screen(&ui);
    assert_eq!(field_text(&ui, "New class"), "1A", "a refused name is not lost");
    replace_text(&ui, "3C");
    press_enter(&ui);
    assert_eq!(field_text(&ui, "New class"), "", "an accepted one empties the field");

    click(&ui, "1A");
    type_into(&ui, "Add pupil…", "   ");
    press_enter(&ui);
    error_on_screen(&ui);
    assert_eq!(field_text(&ui, "Add pupil…"), "   ");
    replace_text(&ui, "Carla");
    press_enter(&ui);
    assert_eq!(field_text(&ui, "Add pupil…"), "");
}

#[test]
fn regression_the_weight_tag_follows_each_languages_percent_style() {
    let ui = seeded();
    assert!(shows_text(&ui, "100%"));
    open_settings(&ui);
    click(&ui, "Galego");
    click(&ui, "1A");
    assert!(shows_text(&ui, "100 %"));
    assert!(!shows_text(&ui, "100%"));
}

// ---------------------------------------------------------------------------------------------
// Pupils in no class

/// Class "1A" with Ana, and Carla, who left her only class: she is in none now.
fn with_a_pupil_in_no_class() -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let first = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, first, "Ana").unwrap();
        let second = classes::create_class(connection, "2B").unwrap();
        let carla = pupils::add_new_pupil(connection, second, "Carla").unwrap();
        pupils::remove_from_class(connection, second, carla).unwrap();
        classes::delete_class(connection, second).unwrap();
    }
    state.shutdown();
    Ui::open(fixture)
}

fn open_the_pupil_menu(ui: &Ui) {
    click(ui, "More actions");
    click(ui, "Add from another class…");
}

#[test]
fn a_pupil_in_no_class_is_marked_and_deleted_after_asking() {
    let ui = with_a_pupil_in_no_class();
    open_the_pupil_menu(&ui);
    assert!(shows_text(&ui, "no class"), "marked in the list");
    ui.dialogs.answer_next(false);
    click(&ui, "Delete Carla");
    assert_eq!(ui.dialogs.questions.borrow()[0], english(&Message::ConfirmDeletePupil { pupil: "Carla" }));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "a cancelled question deletes nobody");
    assert!(shows(&ui, "Delete Carla"));
    ui.dialogs.answer_next(true);
    click(&ui, "Delete Carla");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 1);
    assert!(gradebook(&ui).get_addable_pupils().row_count() == 0);
    assert!(!shows(&ui, "Delete Carla"), "and the emptied menu closes");
}

#[test]
fn a_pupil_in_another_class_cannot_be_deleted_from_the_menu() {
    let ui = with_second_class();
    open_the_pupil_menu(&ui);
    assert!(shows(&ui, "Carla"));
    assert!(!shows(&ui, "Delete Carla"));
    assert!(!shows_text(&ui, "no class"));
}

#[test]
fn deleting_a_pupil_who_has_joined_a_class_meanwhile_is_refused_and_shown() {
    let ui = with_a_pupil_in_no_class();
    open_the_pupil_menu(&ui);
    {
        let state = ui.session.state.borrow();
        let connection = state.connection().unwrap();
        let carla = pupils::pupils_in_no_class(connection).unwrap()[0].id;
        let class = classes::list_classes(connection).unwrap()[0].id;
        pupils::add_existing_pupil(connection, class, carla).unwrap();
    }
    ui.dialogs.answer_next(true);
    click(&ui, "Delete Carla");
    assert_eq!(error_on_screen(&ui), error_text(&AppError::NotEditable));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "nobody was deleted");
}
