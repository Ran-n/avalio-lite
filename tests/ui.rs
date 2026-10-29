// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:06.576798
//+ Revised: 	2026/10/29 01:58:37.957536
// -----------------------------------------------------------------------------

//! The real window, wired as in the app, driven on Slint's headless backend: elements are found
//! by their accessible labels and clicked with pointer events, and text is typed as key events.
//! Dialogs are answered by `FakeDialogs`; data is checked in the database the app has open.

mod common;

use std::fs;

use avalio_lite::config::{DATABASE_FILE_NAME, LOG_FILE_NAME, Preferences, ThemeChoice};
use avalio_lite::db;
use avalio_lite::error::{AppError, Folder};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{classes, columns, pupils};
use avalio_lite::i18n::{self, Language, Message};
use avalio_lite::ui::{Gradebook, SetupNotice, Theme, Tone};
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

/// The settings page, scrolled down to the database and log folders.
fn open_folder_settings(ui: &Ui) {
    open_settings(ui);
    find(ui, "Galego").scroll(0.0, -600.0);
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
    assert_eq!(gradebook(&ui).get_error_message(), english(&Message::LockNeedsPin));
}

// ---------------------------------------------------------------------------------------------
// Settings: PIN

#[test]
fn setting_a_pin_encrypts_the_database() {
    let ui = seeded();
    open_settings(&ui);
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
    open_settings(&ui);
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
    open_settings(&ui);
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
    open_settings(&ui);

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
    assert!(!gradebook(&ui).get_error_message().is_empty());
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
    assert_eq!(gradebook(&ui).get_error_message(), error_text(&AppError::EmptyName));
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
    assert_eq!(gradebook(&ui).get_error_message(), error_text(&AppError::DuplicateClassName("1A".into())));
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
    assert!(!gradebook(&ui).get_error_message().is_empty());
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
    assert_eq!(gradebook(&ui).get_error_message(), error_text(&AppError::NoDatabaseOpen));
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
    assert_eq!(gradebook(&ui).get_error_message(), english(&Message::LockNeedsPin));
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
    open_settings(&ui);
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
