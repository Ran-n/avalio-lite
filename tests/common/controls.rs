// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:06.450073
//+ Revised: 	2026/10/28 15:53:06.450073
// -----------------------------------------------------------------------------

//! Finding the app's controls by their accessible labels, as a screen reader names them. Shared by
//! the headless UI tests and the desktop test driver (`examples/app_driver.rs`).

use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementQuery, ElementRoot};

/// Controls labelled `label`, in the order they appear. The plain text inside a control carries
/// the same label, so text elements are skipped; the search also visits some controls more than
/// once, so repeats of the same control are dropped.
pub fn labelled(root: &impl ElementRoot, label: &str) -> Vec<ElementHandle> {
    let label = label.to_owned();
    let found = ElementQuery::from_root(root)
        .match_descendants()
        .match_predicate(move |element| {
            element.accessible_label().is_some_and(|text| text == label.as_str())
                && element.accessible_role() != Some(AccessibleRole::Text)
        })
        .find_all();
    let mut unique: Vec<ElementHandle> = Vec::new();
    for element in found {
        let place = (element.type_name(), element.absolute_position(), element.size());
        if !unique.iter().any(|seen| (seen.type_name(), seen.absolute_position(), seen.size()) == place) {
            unique.push(element);
        }
    }
    unique
}
