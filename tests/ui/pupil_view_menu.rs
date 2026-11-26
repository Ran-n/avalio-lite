// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:12:06.556897
//+ Revised: 	2026/11/25 11:43:18.137887
// -----------------------------------------------------------------------------

//! The pupil view's menu: the pupils it lists, by mouse and keyboard, and the way back to the sheet.

use crate::common::{Fixture, Ui};
use crate::driver::{click, elements, find, key, press_enter, shows, shows_text};
use crate::fixtures::with_second_class;
use crate::pupil_screens::{
    assert_nothing_on_screen_has, open, pupil_view, sample, sample_photo, sample_with_pin, shown_name, stage,
};
use crate::screens::error_on_screen;
use avalio_lite::gradebook::{classes, pupils};
use avalio_lite::ui::PupilViewStage;
use slint::platform::Key;
use slint::{ComponentHandle, Model};

#[test]
fn the_menu_lists_the_classes_pupils_with_no_grades() {
    let ui = sample();
    open(&ui);
    for pupil in ["Fernández, Uxía", "Iglesias, Carla", "Otero, Brais", "Pérez, Ana", "Souto, Daniel"] {
        assert!(shows(&ui, pupil), "{pupil}");
    }
    assert_nothing_on_screen_has(&ui, &["Exam 1", "Final grade", "Resit in June", "Class average", "passing"]);
    assert!(!shows(&ui, "Settings") && !shows(&ui, "More actions"), "the sidebar and the sheet are gone");
}

/// Sorted by surname, not in the sheet's order, so a pupil is easy to find.
#[test]
fn the_menu_is_sorted_by_surname() {
    let ui = sample();
    open(&ui);
    let names: Vec<String> = pupil_view(&ui).get_pupils().iter().map(|pupil| pupil.listed_name.to_string()).collect();
    assert_eq!(names, ["Fernández, Uxía", "Iglesias, Carla", "Otero, Brais", "Pérez, Ana", "Souto, Daniel"]);
}

#[test]
fn pupils_that_cannot_be_listed_are_reported_on_the_menu() {
    let ui = sample();
    ui.session.state.borrow().connection().unwrap().execute_batch("DROP TABLE class_pupils").unwrap();
    open(&ui);
    error_on_screen(&ui);
    assert!(!shows_text(&ui, "This class has no pupils yet."), "nobody knows that");
    click(&ui, "Back to the sheet");
    assert_eq!(stage(&ui), PupilViewStage::Off);
}

#[test]
fn pupils_can_be_picked_in_any_order() {
    let ui = sample();
    open(&ui);
    for (listed, name) in
        [("Souto, Daniel", "Daniel Souto"), ("Pérez, Ana", "Ana Pérez"), ("Otero, Brais", "Brais Otero")]
    {
        click(&ui, listed);
        assert!(shows_text(&ui, name));
        click(&ui, "Back to the list");
        assert_eq!(stage(&ui), PupilViewStage::Menu);
    }
}

#[test]
fn the_menu_goes_back_to_the_sheet() {
    let ui = sample();
    open(&ui);
    click(&ui, "Back to the sheet");
    assert_eq!(stage(&ui), PupilViewStage::Off);
    assert!(shows(&ui, "More actions") && shows(&ui, "Settings"));
}

#[test]
fn a_class_with_no_pupils_says_so() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    classes::create_class(state.connection().unwrap(), "1A").unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    open(&ui);
    assert!(shows_text(&ui, "This class has no pupils yet."));
}

#[test]
fn only_the_open_classes_pupils_are_offered() {
    let ui = with_second_class();
    open(&ui);
    assert!(shows(&ui, "Ana"));
    assert_nothing_on_screen_has(&ui, &["Carla", "2B"]);
}

#[test]
fn the_menu_warns_when_no_pin_guards_it() {
    let with_none = "Each pupil sees only their own grades. With no PIN set, anyone at the computer can open the others; set a PIN in Settings to prevent it.";
    let with_one =
        "Each pupil sees only their own grades. Showing a pupil and going back to the sheet ask for your PIN.";
    let ui = sample();
    open(&ui);
    assert!(shows_text(&ui, with_none) && !shows_text(&ui, with_one));
    drop(ui);
    let ui = sample_with_pin();
    open(&ui);
    assert!(shows_text(&ui, with_one) && !shows_text(&ui, with_none));
}

#[test]
fn the_menu_works_from_the_keyboard() {
    let ui = sample();
    open(&ui);
    // the first Tab reaches Back to the sheet, the second the first pupil
    key(&ui, Key::Tab);
    key(&ui, Key::Tab);
    press_enter(&ui);
    assert_eq!(shown_name(&ui), "Uxía Fernández");
}

/// Forty pupils don't fit; Tab brings the button it reaches into view.
#[test]
fn tab_scrolls_the_menu_to_the_pupil_it_reaches() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "1A").unwrap();
        for number in 10..90 {
            pupils::add_new_pupil(connection, class, &format!("Pupil {number}"), "").unwrap();
        }
    }
    state.shutdown();
    let ui = Ui::open(fixture);
    ui.session.window.window().set_size(slint::LogicalSize::new(1000.0, 600.0));
    open(&ui);
    let bottom = |ui: &Ui| {
        let last = elements(ui, "Pupil 89");
        last.first().map(|button| button.absolute_position().y + button.size().height)
    };
    assert!(bottom(&ui).is_none_or(|bottom| bottom > 600.0), "the last pupil is below the window");
    for _ in 0..81 {
        key(&ui, Key::Tab);
    }
    assert!(bottom(&ui).is_some_and(|bottom| bottom <= 600.0), "in view: {:?}", bottom(&ui));
    for _ in 0..79 {
        key(&ui, Key::Backtab);
    }
    let first = elements(&ui, "Pupil 10");
    assert!(first[0].absolute_position().y > 0.0, "and back up");
    click(&ui, "Pupil 10");
    assert_eq!(shown_name(&ui), "Pupil 10");
}

/// The pupil view's list shows the picture, the number and the name.
#[test]
fn the_list_of_pupils_shows_picture_number_and_name() {
    let ui = sample();
    click(&ui, "Picture of Uxía Fernández");
    ui.dialogs.pick_file_next(Some(&sample_photo(&ui.fixture)));
    click(&ui, "Choose a picture…");
    click(&ui, "Close");
    open(&ui);
    let cards: Vec<_> = pupil_view(&ui).get_pupils().iter().collect();
    let shown: Vec<(i32, &str, bool)> =
        cards.iter().map(|card| (card.number, card.listed_name.as_str(), card.avatar.size().width > 0)).collect();
    let expected = [
        (1, "Fernández, Uxía", true),
        (2, "Iglesias, Carla", false),
        (3, "Otero, Brais", false),
        (4, "Pérez, Ana", false),
        (5, "Souto, Daniel", false),
    ];
    assert_eq!(shown, expected);
    assert_eq!(cards[1].initials, "CI", "initials where there is no picture");
    assert_eq!(find(&ui, "Otero, Brais").accessible_description().unwrap(), "Number 3");
}
