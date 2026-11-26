// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:19.228698
//+ Revised: 	2026/11/25 11:43:18.455785
// -----------------------------------------------------------------------------

//! What asks for the PIN in the pupil view, and what doesn't.

use crate::driver::{PIN, click, shows, shows_text};
use crate::fixtures::type_pin_on_keypad;
use crate::pupil_screens::{
    assert_nothing_on_screen_has, grades, open, open_for, pupil_view, sample, sample_with_pin, shown_name, stage,
    unlock_with,
};
use avalio_lite::ui::{PupilView, PupilViewStage};

#[test]
fn opening_the_list_and_going_back_to_it_never_ask() {
    let ui = sample_with_pin();
    open(&ui);
    click(&ui, "Pérez, Ana");
    unlock_with(&ui, PIN);
    assert_eq!(stage(&ui), PupilViewStage::Grades);
    click(&ui, "Back to the list");
    assert_eq!(stage(&ui), PupilViewStage::Menu);
}

#[test]
fn every_pupil_picked_asks_even_the_same_one_again() {
    let ui = sample_with_pin();
    open(&ui);
    for (listed, name) in
        [("Pérez, Ana", "Ana Pérez"), ("Souto, Daniel", "Daniel Souto"), ("Souto, Daniel", "Daniel Souto")]
    {
        click(&ui, listed);
        assert_eq!(stage(&ui), PupilViewStage::Pin);
        assert!(shows_text(&ui, &format!("Your PIN, to show the grades of {name}")));
        unlock_with(&ui, PIN);
        assert_eq!(shown_name(&ui), name);
        click(&ui, "Back to the list");
    }
}

#[test]
fn next_asks_and_shows_the_pupil_after_by_number() {
    let ui = sample_with_pin();
    open_for(&ui, "Iglesias, Carla");
    click(&ui, "Next pupil");
    assert_eq!(stage(&ui), PupilViewStage::Pin);
    assert!(shows_text(&ui, "Your PIN, to show the grades of Brais Otero"));
    assert_nothing_on_screen_has(&ui, &["Carla", "Exam 1", "Pérez"]);
    unlock_with(&ui, PIN);
    assert_eq!(shown_name(&ui), "Brais Otero");
}

#[test]
fn leaving_the_list_asks() {
    let ui = sample_with_pin();
    open(&ui);
    click(&ui, "Back to the sheet");
    assert_eq!(stage(&ui), PupilViewStage::Pin);
    assert!(shows_text(&ui, "Your PIN, to go back to the sheet"));
    assert!(!shows(&ui, "More actions"));
    unlock_with(&ui, PIN);
    assert_eq!(stage(&ui), PupilViewStage::Off);
    assert!(shows(&ui, "More actions"));
}

#[test]
fn giving_up_on_the_pin_goes_back_to_the_list() {
    let ui = sample_with_pin();
    open(&ui);
    for asks in ["Pérez, Ana", "Back to the sheet"] {
        click(&ui, asks);
        type_pin_on_keypad(&ui, "12");
        click(&ui, "Back to the list");
        assert_eq!(stage(&ui), PupilViewStage::Menu);
        let view = pupil_view(&ui);
        assert!(view.get_pin().is_empty() && view.get_wanted_name().is_empty(), "nothing typed is left");
        assert_eq!(view.get_wanted_pupil(), -1);
    }
}

#[test]
fn with_no_pin_nothing_asks() {
    let ui = sample();
    open_for(&ui, "Iglesias, Carla");
    click(&ui, "Next pupil");
    assert_eq!(shown_name(&ui), "Brais Otero");
    click(&ui, "Back to the list");
    click(&ui, "Back to the sheet");
    assert_eq!(stage(&ui), PupilViewStage::Off);
}

#[test]
fn no_callback_skips_the_pin() {
    let ui = sample_with_pin();
    let view = pupil_view(&ui);
    let everything = |except_open: bool| {
        for invoke in [PupilView::invoke_next, PupilView::invoke_leave] {
            invoke(&view);
        }
        view.invoke_show_pupil(1);
        if !except_open {
            view.invoke_open();
        }
    };
    everything(true);
    view.invoke_back_to_list();
    view.invoke_unlock(PIN.into());
    assert_eq!(stage(&ui), PupilViewStage::Off, "nothing but open starts it");

    open_for(&ui, "Pérez, Ana");
    view.invoke_leave();
    view.invoke_show_pupil(2);
    view.invoke_unlock(PIN.into());
    view.invoke_open();
    assert_eq!(stage(&ui), PupilViewStage::Grades);
    assert_eq!(shown_name(&ui), "Ana Pérez", "only next and back do anything here");

    click(&ui, "Next pupil");
    everything(false);
    assert_eq!(stage(&ui), PupilViewStage::Pin);
    assert!(grades(&ui).is_empty());
    assert_eq!(view.get_wanted_name(), "Daniel Souto", "and what it is for can't be swapped");
    view.invoke_unlock("".into());
    assert_eq!(stage(&ui), PupilViewStage::Pin, "an empty PIN is a wrong one");
}
