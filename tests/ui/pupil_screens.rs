// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:12:06.246433
//+ Revised: 	2026/11/25 11:43:17.276660
// -----------------------------------------------------------------------------

//! What the pupil view's tests share: the sample class with and without a PIN, opening the view,
//! and reading everything the screen says.

use crate::common::{Fixture, Ui, sample_class};
use crate::driver::{PIN, click};
use crate::fixtures::type_pin_on_keypad;
use avalio_lite::ui::{GradeItem, PupilView, PupilViewStage};
use i_slint_backend_testing::ElementQuery;
use slint::{ComponentHandle, Model};

pub(crate) fn pupil_view(ui: &Ui) -> PupilView<'_> {
    ui.session.window.global::<PupilView>()
}

pub(crate) fn stage(ui: &Ui) -> PupilViewStage {
    pupil_view(ui).get_stage()
}

/// The sample class ("2º ESO A", five pupils), behind `PIN` when asked.
pub(crate) fn sample_fixture(pin: Option<&str>) -> Fixture {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    sample_class(state.connection().unwrap());
    if let Some(pin) = pin {
        state.set_pin(pin, pin).unwrap();
    }
    state.shutdown();
    fixture
}

pub(crate) fn sample() -> Ui {
    Ui::open(sample_fixture(None))
}

pub(crate) fn sample_with_pin() -> Ui {
    let ui = Ui::open(sample_fixture(Some(PIN)));
    type_pin_on_keypad(&ui, PIN);
    click(&ui, "Unlock");
    assert!(!ui.session.window.get_locked());
    ui
}

pub(crate) fn open(ui: &Ui) {
    click(ui, "More actions");
    click(ui, "Show a pupil their grades…");
    assert_eq!(stage(ui), PupilViewStage::Menu);
}

/// Types `pin` on the keypad of the page that asks for it, and presses ✓.
pub(crate) fn unlock_with(ui: &Ui, pin: &str) {
    type_pin_on_keypad(ui, pin);
    click(ui, "Unlock");
}

/// Opens the view on `pupil`, typing the PIN when there is one.
pub(crate) fn open_for(ui: &Ui, pupil: &str) {
    open(ui);
    click(ui, pupil);
    if stage(ui) == PupilViewStage::Pin {
        unlock_with(ui, PIN);
    }
    assert_eq!(stage(ui), PupilViewStage::Grades);
}

/// The pupil whose grades show; empty when there is none.
pub(crate) fn shown_name(ui: &Ui) -> String {
    pupil_view(ui).get_pupil().full_name.to_string()
}

/// A photo to choose as a pupil's picture, in the fixture folder.
pub(crate) fn sample_photo(fixture: &Fixture) -> std::path::PathBuf {
    let path = fixture.path("photo.png");
    image::RgbImage::from_pixel(60, 40, image::Rgb([200, 120, 40])).save(&path).unwrap();
    path
}

pub(crate) fn grades(ui: &Ui) -> Vec<GradeItem> {
    pupil_view(ui).get_grades().iter().collect()
}

/// Everything on screen says: every label and value, of controls and of plain texts.
pub(crate) fn everything_on_screen(ui: &Ui) -> Vec<String> {
    ElementQuery::from_root(&ui.session.window)
        .match_descendants()
        .find_all()
        .iter()
        .flat_map(|element| [element.accessible_label(), element.accessible_value(), element.accessible_description()])
        .flatten()
        .map(|text| text.to_string())
        .collect()
}

pub(crate) fn assert_nothing_on_screen_has(ui: &Ui, forbidden: &[&str]) {
    let shown = everything_on_screen(ui);
    assert!(!shown.is_empty(), "the screen is read");
    for text in forbidden {
        assert!(!shown.iter().any(|label| label.contains(text)), "{text:?} is on screen: {shown:?}");
    }
}
