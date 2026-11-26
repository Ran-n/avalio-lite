// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 01:29:26.380283
//+ Revised: 	2026/11/24 17:54:05.246878
// -----------------------------------------------------------------------------

//! The totals the sheet shows under the class name: the class average with its pass and fail
//! counts, and the weight totals. They change with the grading display, not with the rows, so they
//! are kept apart from the rest of the sheet's redraw.

use slint::ComponentHandle;

use crate::gradebook::compute::{
    DISPLAY_DECIMALS, bonus_weight_total, grade_summary, grades_are_provisional, round_to, weight_total,
};
use crate::gradebook::model::{Column, Sheet};
use crate::gradebook::ui::items::number;
use crate::ui::{AppWindow, Gradebook};

/// The class average and pass / fail counts under the class name.
pub fn show_summary(window: &AppWindow, loaded: &Sheet) {
    let values: Vec<Vec<String>> = loaded.rows.iter().map(|row| row.values.clone()).collect();
    let summary = grade_summary(&loaded.columns, &values);
    let gradebook = window.global::<Gradebook>();
    gradebook.set_has_summary(summary.is_some());
    if let Some(summary) = summary {
        gradebook.set_class_average(number(round_to(summary.average, DISPLAY_DECIMALS)).into());
        gradebook.set_passing(i32::try_from(summary.passing).unwrap_or(i32::MAX));
        gradebook.set_failing(i32::try_from(summary.failing).unwrap_or(i32::MAX));
    }
}

/// The weight totals of the columns, and whether the weights are complete enough to grade by.
pub fn show_weight_total(window: &AppWindow, columns: &[Column]) {
    let gradebook = window.global::<Gradebook>();
    gradebook.set_weight_total(number(weight_total(columns)).into());
    gradebook.set_bonus_total(number(bonus_weight_total(columns)).into());
    gradebook.set_weights_complete(!grades_are_provisional(columns));
}
