// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:12:06.855726
//+ Revised: 	2026/11/25 13:46:17.188474
// -----------------------------------------------------------------------------

//! The pupil view while the rest of the app goes on: updates, the licence, a pupil removed, a database
//! that can't be read, and the app closing.

use crate::common::Ui;
use crate::driver::{PIN, click, count, english, error_text, press_enter, shows, shows_text, type_text, with_control};
use crate::pupil_screens::{assert_nothing_on_screen_has, open, open_for, sample, sample_with_pin, stage, unlock_with};
use crate::screens::{automatic_check, error_on_screen, licence, next_version, release, update};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::{classes, pupils};
use avalio_lite::i18n::{Message, SheetMessage};
use avalio_lite::ui::{PupilViewStage, UpdateBanner};

/// The update's restart would open the sheet in front of the pupil when there is no PIN.
#[test]
fn it_does_not_start_while_an_update_downloads() {
    let ui = sample();
    update(&ui).set_downloading(true);
    click(&ui, "More actions");
    click(&ui, "Show a pupil their grades…");
    assert_eq!(stage(&ui), PupilViewStage::Off);
    assert_eq!(error_on_screen(&ui), english(&Message::Sheet(SheetMessage::PupilViewWhileUpdating)));
    update(&ui).set_downloading(false);
    open(&ui);
    assert!(ui.session.window.get_error_message().is_empty(), "the message is gone");
}

#[test]
fn a_new_version_found_meanwhile_is_offered_only_afterwards() {
    let ui = sample();
    open_for(&ui, "Pérez, Ana");
    ui.releases.set(Ok(release(&next_version())));
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
    assert!(!shows(&ui, "Update and restart"));
    click(&ui, "Back to the list");
    click(&ui, "Back to the sheet");
    assert!(shows(&ui, "Update and restart"));
}

#[test]
fn losing_the_licence_meanwhile_shows_no_grades() {
    let ui = sample_with_pin();
    open_for(&ui, "Pérez, Ana");
    click(&ui, "Next pupil");
    licence(&ui).set_licensed(false);
    assert_nothing_on_screen_has(&ui, &["Pérez", "Souto", "Exam 1", "Your PIN"]);
    licence(&ui).set_licensed(true);
    assert!(shows_text(&ui, "Your PIN, to show the grades of Daniel Souto"), "the PIN is back, not the sheet");
    type_text(&ui, PIN);
    press_enter(&ui);
    assert_eq!(stage(&ui), PupilViewStage::Grades, "and its field has the keyboard");
}

#[test]
fn a_pupil_removed_meanwhile_is_said_and_leaves_the_menu() {
    let ui = sample();
    open(&ui);
    {
        let state = ui.session.state.borrow();
        let connection = state.connection().unwrap();
        let class = classes::list_classes(connection).unwrap()[0].id;
        let ana =
            pupils::list_class_pupils(connection, class).unwrap().into_iter().find(|pupil| pupil.first_name == "Ana");
        pupils::remove_from_class(connection, class, ana.unwrap().id).unwrap();
    }
    click(&ui, "Pérez, Ana");
    assert_eq!(stage(&ui), PupilViewStage::Menu);
    assert_eq!(error_on_screen(&ui), english(&Message::Sheet(SheetMessage::PupilNotInClass)));
    assert!(!shows(&ui, "Pérez, Ana"));
    click(&ui, "Otero, Brais");
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM class_pupils"), 4);
}

#[test]
fn grades_that_cannot_be_read_are_reported_on_the_menu() {
    let ui = sample();
    open(&ui);
    ui.session.state.borrow().connection().unwrap().execute_batch("DROP TABLE cells").unwrap();
    click(&ui, "Pérez, Ana");
    assert_eq!(stage(&ui), PupilViewStage::Menu);
    error_on_screen(&ui);
}

#[test]
fn with_no_database_open_it_says_so() {
    let ui = sample();
    open(&ui);
    ui.session.state.borrow_mut().shutdown();
    click(&ui, "Pérez, Ana");
    assert_eq!(error_on_screen(&ui), error_text(&AppError::NoDatabaseOpen));
    assert_eq!(stage(&ui), PupilViewStage::Menu);
}

/// The message of the sheet's last action isn't carried into the menu, nor the menu's back.
#[test]
fn messages_stay_on_the_page_they_belong_to() {
    let ui = sample();
    with_control(&ui, "s");
    assert!(!ui.session.window.get_status_message().is_empty());
    open(&ui);
    assert!(ui.session.window.get_status_message().is_empty());
    ui.session.window.set_error_message("left over".into());
    click(&ui, "Back to the sheet");
    assert!(ui.session.window.get_error_message().is_empty());
}

/// Restarted, the app opens as it always does: on the lock screen when there is a PIN.
#[test]
fn closing_the_app_meanwhile_opens_on_the_lock_screen() {
    let ui = sample_with_pin();
    open_for(&ui, "Pérez, Ana");
    let ui = ui.restart();
    assert!(ui.session.window.get_locked());
    assert_eq!(stage(&ui), PupilViewStage::Off);
}

/// With a PIN, a pupil removed before or while it is typed shows no one's grades either.
#[test]
fn a_pupil_removed_while_the_pin_is_typed_is_said_on_the_list() {
    let ui = sample_with_pin();
    open(&ui);
    let remove = |ui: &Ui, surname: &str| {
        let state = ui.session.state.borrow();
        let sql = "DELETE FROM class_pupils WHERE pupil_id = (SELECT id FROM pupils WHERE surname = ?1)";
        state.connection().unwrap().execute(sql, [surname]).unwrap();
    };
    click(&ui, "Pérez, Ana");
    remove(&ui, "Pérez");
    unlock_with(&ui, PIN);
    assert_eq!(stage(&ui), PupilViewStage::Menu);
    assert_eq!(error_on_screen(&ui), english(&Message::Sheet(SheetMessage::PupilNotInClass)));
    remove(&ui, "Otero");
    click(&ui, "Otero, Brais");
    assert_eq!(stage(&ui), PupilViewStage::Menu, "no PIN is asked for a pupil who isn't there");
    assert!(!shows(&ui, "Otero, Brais") && shows(&ui, "Souto, Daniel"));
}

/// The next pupil can't be read: said on the list, with the last pupil's grades already gone.
#[test]
fn a_next_pupil_that_cannot_be_read_is_reported_on_the_list() {
    let ui = sample();
    open_for(&ui, "Iglesias, Carla");
    ui.session.state.borrow_mut().shutdown();
    click(&ui, "Next pupil");
    assert_eq!(error_on_screen(&ui), error_text(&AppError::NoDatabaseOpen));
    assert_nothing_on_screen_has(&ui, &["Carla", "Exam 1"]);
}

#[test]
fn a_pick_that_cannot_be_read_is_reported_before_the_pin() {
    let ui = sample_with_pin();
    open(&ui);
    ui.session.state.borrow().connection().unwrap().execute_batch("DROP TABLE class_pupils").unwrap();
    click(&ui, "Pérez, Ana");
    assert_eq!(stage(&ui), PupilViewStage::Menu);
    error_on_screen(&ui);
}

/// The pupil is gone and the sheet can't be read either: the failure is said, with no grades shown.
#[test]
fn a_pupil_gone_from_a_sheet_that_cannot_be_read_is_reported() {
    let ui = sample();
    open(&ui);
    let broken = "DELETE FROM class_pupils WHERE pupil_id = (SELECT id FROM pupils WHERE surname = 'Pérez');
                  DROP TABLE cells;";
    ui.session.state.borrow().connection().unwrap().execute_batch(broken).unwrap();
    click(&ui, "Pérez, Ana");
    assert_eq!(stage(&ui), PupilViewStage::Menu);
    error_on_screen(&ui);
}
