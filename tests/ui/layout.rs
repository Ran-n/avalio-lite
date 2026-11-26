// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.941290
//+ Revised: 	2026/11/23 13:00:31.941290
// -----------------------------------------------------------------------------

//! Layout regressions: each text fits its window at the smallest size and in Galician.

use crate::common::{Fixture, Ui};
use crate::driver::{click, elements, find};
use crate::fixtures::seeded;
use avalio_lite::i18n::Language;
use i_slint_backend_testing::ElementHandle;
use slint::ComponentHandle;

/// In Galician at the window's minimum width, the sheet's title row (class name, summary line and
/// the three buttons) was wider than the window: "+ Engadir columna" was cut off and "⋯" (with
/// "Delete class" behind it) was pushed out of sight. Found by the desktop tests' Galician check.
#[test]
fn regression_sheet_actions_fit_the_smallest_window_in_galician() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    crate::common::sample_class(state.connection().unwrap());
    state.set_language(Language::Galician).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    ui.session.window.window().set_size(slint::LogicalSize::new(1000.0, 600.0));
    for english in ["+ Add pupil", "+ Add column", "More actions"] {
        let label = crate::common::galician(english);
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
    crate::common::sample_class(state.connection().unwrap());
    state.set_language(Language::Galician).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    ui.session.window.window().set_size(slint::LogicalSize::new(1000.0, 600.0));
    click(&ui, "Exam 1");
    let close = find(&ui, &crate::common::galician("Close"));
    let panel_right = close.absolute_position().x + close.size().width + 24.0;
    for english in
        ["Move left (Ctrl+←)", "Move right (Ctrl+→)", "Delete", "Previous column (Alt+←)", "Next column (Alt+→)"]
    {
        let label = crate::common::galician(english);
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
