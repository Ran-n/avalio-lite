// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:17.247074
//+ Revised: 	2026/11/25 11:43:09.074727
// -----------------------------------------------------------------------------

//! What the pupil view's pages hold: the list of pupils with their numbers and pictures, and one
//! pupil's grades. Read from the database each time, and cleared as soon as the page is left.

use rusqlite::Connection;
use slint::{ComponentHandle, Image, ModelRc, SharedString};

use crate::error::{AppError, Result};
use crate::gradebook::model::Pupil;
use crate::gradebook::ui::avatars::{Size, class_avatars, initials, pupil_avatar};
use crate::gradebook::ui::items::{number, to_ui_tone};
use crate::gradebook::ui::view::{current_class_id, ui_id};
use crate::gradebook::{pupils, sheet};
use crate::i18n;
use crate::pupil_view::{Grade, pupil_grades};
use crate::ui::{AppWindow, GradeItem, PupilCard, PupilView};
use crate::window::{SharedState, view};

/// One pupil and their grades, ready for the page.
pub struct Shown {
    card: PupilCard,
    has_next: bool,
    grades: Vec<Grade>,
}

/// The open class's pupils, by surname, each with their number.
fn listed(window: &AppWindow, connection: &Connection) -> Result<Vec<(i32, Pupil)>> {
    Ok((1..).zip(pupils::class_list(connection, current_class_id(window))?).collect())
}

/// Runs `read` on the open database.
fn reading<T>(state: &SharedState, read: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
    let state = state.borrow();
    read(state.connection().ok_or(AppError::NoDatabaseOpen)?)
}

pub fn fill_menu(window: &AppWindow, state: &SharedState) -> Result<()> {
    let cards: Vec<PupilCard> = reading(state, |connection| {
        let mut avatars = class_avatars(connection, current_class_id(window), Size::Card)?;
        let listed = listed(window, connection)?;
        Ok(listed
            .iter()
            .map(|(number, pupil)| card(*number, pupil, avatars.remove(&pupil.id).unwrap_or_default()))
            .collect())
    })?;
    window.global::<PupilView>().set_pupils(view::model(cards));
    Ok(())
}

fn card(number: i32, pupil: &Pupil, avatar: Image) -> PupilCard {
    PupilCard {
        id: ui_id(pupil.id),
        number,
        listed_name: pupil.listed_name().into(),
        full_name: pupil.full_name().into(),
        initials: initials(pupil).into(),
        avatar,
    }
}

/// The pupil's name; `None` when they aren't in the open class.
pub fn full_name(window: &AppWindow, state: &SharedState, pupil_id: i32) -> Result<Option<String>> {
    let listed = reading(state, |connection| listed(window, connection))?;
    Ok(listed.iter().find(|(_, pupil)| ui_id(pupil.id) == pupil_id).map(|(_, pupil)| pupil.full_name()))
}

/// The pupil after `pupil_id` by number; `None` after the last, or when `pupil_id` left the class.
pub fn next_pupil(window: &AppWindow, state: &SharedState, pupil_id: i32) -> Result<Option<i32>> {
    let listed = reading(state, |connection| listed(window, connection))?;
    Ok(listed.iter().map(|(_, pupil)| ui_id(pupil.id)).skip_while(|id| *id != pupil_id).nth(1))
}

/// The pupil and their grades in the open class; `None` when they aren't in it.
pub fn load_grades(window: &AppWindow, state: &SharedState, pupil_id: i32) -> Result<Option<Shown>> {
    reading(state, |connection| {
        let listed = listed(window, connection)?;
        let Some((number, pupil)) = listed.iter().find(|(_, pupil)| ui_id(pupil.id) == pupil_id) else {
            return Ok(None);
        };
        let loaded = sheet::load_sheet(connection, current_class_id(window))?;
        let card = card(*number, pupil, pupil_avatar(connection, pupil.id, Size::Card)?);
        let has_next = listed.last().is_some_and(|(last, _)| last > number);
        Ok(pupil_grades(&loaded, pupil.id).map(|grades| Shown { card, has_next, grades }))
    })
}

/// Fills the grades page; the list of the others' names goes.
pub fn fill_grades(window: &AppWindow, shown: Shown) {
    forget(window);
    let pupil_view = window.global::<PupilView>();
    pupil_view.set_pupil(shown.card);
    pupil_view.set_has_next(shown.has_next);
    pupil_view.set_any_provisional(shown.grades.iter().any(|grade| grade.provisional && !grade.text.is_empty()));
    pupil_view.set_grades(view::model(shown.grades.iter().map(grade_item).collect()));
}

fn grade_item(grade: &Grade) -> GradeItem {
    let text = if grade.computed { i18n::localize_number(i18n::current(), &grade.text) } else { grade.text.clone() };
    GradeItem {
        title: grade.title.as_str().into(),
        text: text.into(),
        max_score: grade.max_score.map(number).unwrap_or_default().into(),
        weight: grade.weight.map(|weight| number(weight.percent)).unwrap_or_default().into(),
        bonus: grade.weight.is_some_and(|weight| weight.bonus),
        tone: to_ui_tone(grade.tone),
        provisional: grade.provisional,
    }
}

/// Nothing of any pupil stays in the window, nor of a PIN half typed or whom it was for.
pub fn forget(window: &AppWindow) {
    let pupil_view = window.global::<PupilView>();
    pupil_view.set_pupils(ModelRc::default());
    pupil_view.set_pupil(PupilCard::default());
    pupil_view.set_has_next(false);
    pupil_view.set_grades(ModelRc::default());
    pupil_view.set_any_provisional(false);
    pupil_view.set_wanted_pupil(super::LEAVING);
    pupil_view.set_wanted_name(SharedString::new());
    pupil_view.set_pin(SharedString::new());
    pupil_view.set_pin_error(SharedString::new());
}
