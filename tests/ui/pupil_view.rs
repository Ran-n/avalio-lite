// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:12:06.404167
//+ Revised: 	2026/11/25 11:43:17.565996
// -----------------------------------------------------------------------------

//! The pupil view's page: one pupil's own grades, and nothing else on screen.

use crate::common::{Ui, galician};
use crate::driver::{click, gradebook, shows, shows_text};
use crate::fixtures::{seeded, with_second_class};
use crate::pupil_screens::{
    assert_nothing_on_screen_has, everything_on_screen, grades, open_for, pupil_view, sample, sample_fixture,
    shown_name, stage,
};
use avalio_lite::gradebook::{classes, columns};
use avalio_lite::i18n::Language;
use avalio_lite::ui::{PupilViewStage, Tone};
use i_slint_backend_testing::{AccessibleRole, ElementQuery};

#[test]
fn a_pupil_sees_their_own_grades() {
    let ui = sample();
    open_for(&ui, "Otero, Brais");
    assert!(shows_text(&ui, "Brais Otero") && shows_text(&ui, "Number 3 · 2º ESO A"));
    for label in [
        "Exam 1: 6,5 / 10",
        "Lab report: 1 / 5",
        "Project: no mark yet",
        "Final grade: 4.7 / 10",
        "Report card: 5 / 10",
    ] {
        assert!(shows_text(&ui, label), "{label}: {:?}", everything_on_screen(&ui));
    }
    let tones: Vec<Tone> = grades(&ui).iter().map(|grade| grade.tone).collect();
    assert_eq!(tones, [Tone::Pass, Tone::Fail, Tone::Neutral, Tone::Fail, Tone::Pass]);
}

/// The reason the mode exists: nothing of anyone else, of the class or of the teacher's notes.
#[test]
fn nothing_else_is_on_screen_while_a_pupil_looks() {
    let ui = sample();
    open_for(&ui, "Souto, Daniel");
    let others = ["Ana", "Pérez", "Brais", "Otero", "Carla", "Iglesias", "Uxía", "Fernández"];
    assert_nothing_on_screen_has(&ui, &others);
    assert_nothing_on_screen_has(&ui, &["Resit in June", "Notes", "Class average", "passing", "failing", "Settings"]);
    assert_nothing_on_screen_has(&ui, &["8", "6,5", "9,5", "0,5"]);
    assert!(shows_text(&ui, "Exam 1: 4 / 10"), "only Daniel's own mark");
    let buttons: Vec<String> = ElementQuery::from_root(&ui.session.window)
        .match_descendants()
        .match_accessible_role(AccessibleRole::Button)
        .find_all()
        .iter()
        .filter_map(|button| button.accessible_label())
        .map(|label| label.to_string())
        .collect();
    assert!(buttons.iter().all(|label| label == "Back to the list"), "no next after the last pupil: {buttons:?}");
}

#[test]
fn weights_show_when_the_sheet_shows_them() {
    let ui = sample();
    open_for(&ui, "Pérez, Ana");
    assert!(shows_text(&ui, "60%") && shows_text(&ui, "+10% bonus"));
    click(&ui, "Back to the list");
    click(&ui, "Back to the sheet");
    click(&ui, "More actions");
    click(&ui, "Hide weights & scales");
    open_for(&ui, "Pérez, Ana");
    assert!(!shows_text(&ui, "60%") && !shows_text(&ui, "+10% bonus"));
    assert!(shows_text(&ui, "Exam 1: 8 / 10"), "a mark always says what it is out of");
}

#[test]
fn provisional_grades_say_so() {
    let ui = sample();
    {
        let state = ui.session.state.borrow();
        let connection = state.connection().unwrap();
        let class = classes::list_classes(connection).unwrap()[0].id;
        let lab =
            columns::list_columns(connection, class).unwrap().into_iter().find(|column| column.title == "Lab report");
        columns::set_weight(connection, lab.unwrap().id, "20").unwrap();
    }
    open_for(&ui, "Pérez, Ana");
    assert!(shows_text(&ui, "Grades marked with ! are provisional: they can still change."));
    let labels = everything_on_screen(&ui);
    assert!(
        labels.iter().any(|label| label.starts_with("Final grade: ") && label.ends_with("(provisional)")),
        "{labels:?}"
    );
    assert!(shows_text(&ui, "Exam 1: 8 / 10"), "a typed mark isn't provisional");
}

#[test]
fn a_class_with_no_grades_yet_says_so() {
    let ui = with_second_class();
    open_for(&ui, "Ana");
    assert!(shows_text(&ui, "There are no grades in this class yet."));
}

#[test]
fn calculated_grades_have_a_decimal_comma_in_galician() {
    let ui = Ui::open_in(sample_fixture(None), Language::Galician);
    click(&ui, "Máis accións");
    click(&ui, &galician("Show a pupil their grades…"));
    click(&ui, "Otero, Brais");
    assert!(shows_text(&ui, "Final grade: 4,7 / 10"), "{:?}", everything_on_screen(&ui));
    assert!(shows_text(&ui, "Project: aínda sen nota"));
    click(&ui, &galician("Back to the list"));
    assert_eq!(stage(&ui), PupilViewStage::Menu);
}

#[test]
fn going_back_leaves_nothing_of_the_pupil_behind() {
    let ui = sample();
    open_for(&ui, "Pérez, Ana");
    click(&ui, "Back to the list");
    assert!(shown_name(&ui).is_empty());
    assert!(grades(&ui).is_empty());
    assert!(!pupil_view(&ui).get_any_provisional());
}

#[test]
fn the_grades_are_read_afresh_each_time() {
    let ui = seeded();
    open_for(&ui, "Ana");
    assert!(shows_text(&ui, "Exam: no mark yet"));
    click(&ui, "Back to the list");
    click(&ui, "Back to the sheet");
    avalio_lite::ui::Gradebook::invoke_cell_edited(&gradebook(&ui), 0, 2, "7".into());
    open_for(&ui, "Ana");
    assert!(shows_text(&ui, "Exam: 7 / 10"), "{:?}", everything_on_screen(&ui));
}

#[test]
fn looking_at_grades_changes_nothing_in_the_database() {
    let ui = sample();
    let changes = |ui: &Ui| ui.session.state.borrow().connection().unwrap().total_changes();
    let before = changes(&ui);
    open_for(&ui, "Pérez, Ana");
    click(&ui, "Back to the list");
    click(&ui, "Back to the sheet");
    assert_eq!(changes(&ui), before);
}

#[test]
fn next_goes_by_number_up_to_the_last() {
    let ui = sample();
    open_for(&ui, "Fernández, Uxía");
    let mut seen = vec![shown_name(&ui)];
    while shows(&ui, "Next pupil") {
        click(&ui, "Next pupil");
        seen.push(shown_name(&ui));
    }
    assert_eq!(seen, ["Uxía Fernández", "Carla Iglesias", "Brais Otero", "Ana Pérez", "Daniel Souto"]);
    pupil_view(&ui).invoke_next();
    assert_eq!(stage(&ui), PupilViewStage::Menu, "past the last, the list");
}
