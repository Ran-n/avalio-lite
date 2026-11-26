// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 21:48:11.540835
//+ Revised: 	2026/11/26 14:41:30.494766
// -----------------------------------------------------------------------------

//! Where the window opens (`app::placement::place`): the size and place rules on screens of every
//! size and scale, apart from any real screen.

mod common;

use avalio_lite::app::placement::{
    self, Frame, MIN_HEIGHT, MIN_WIDTH, PREFERRED_HEIGHT, PREFERRED_WIDTH, Placement, Shown, WindowPlace, WorkArea,
    place, remembered, restore,
};
use common::Ui;

/// The sizes are written twice, in `ui/app-window.slint` (the window's own) and in placement's
/// constants (what it fits on the screen); a change to one alone would place the window wrong.
#[test]
fn the_window_sizes_match_the_slint_window() {
    let slint = include_str!("../ui/app-window.slint");
    let declared = |property: &str| -> f32 {
        let line = slint
            .lines()
            .map(str::trim)
            .find(|line| line.starts_with(&format!("{property}:")) || line.contains(&format!(" {property}:")))
            .unwrap_or_else(|| panic!("ui/app-window.slint has no {property}"));
        let value = line.rsplit(':').next().unwrap().trim().trim_end_matches(';').trim_end_matches("px");
        value.parse().unwrap_or_else(|_| panic!("{property} isn't a plain length: {line}"))
    };
    assert_eq!(declared("fit-min-width"), avalio_lite::app::placement::MIN_WIDTH);
    assert_eq!(declared("fit-min-height"), avalio_lite::app::placement::MIN_HEIGHT);
    assert_eq!(declared("preferred-width"), avalio_lite::app::placement::PREFERRED_WIDTH);
    assert_eq!(declared("preferred-height"), avalio_lite::app::placement::PREFERRED_HEIGHT);
}
use proptest::prelude::*;
use slint::ComponentHandle;

#[expect(clippy::too_many_arguments, reason = "a shorthand for writing a WorkArea's five fields in a row in the tests")]
fn area(left: i32, top: i32, width: u32, height: u32, scale: f32) -> WorkArea {
    WorkArea { left, top, width, height, scale }
}

fn window(placement: Placement) -> (i32, i32, f32, f32) {
    match placement {
        Placement::Window { x, y, width, height, maximized: false } => (x, y, width, height),
        other => panic!("expected a window, got {other:?}"),
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05
}

/// A Windows 10 frame at 100 %: 8 px borders and a 31 px title bar above them.
const FRAME_100: Frame = Frame { left: 8, top: 31, right: 8, bottom: 8 };

#[test]
fn a_full_hd_screen_gets_the_preferred_size_centred() {
    assert_eq!(window(place(&area(0, 0, 1920, 1040, 1.0), Frame::default())), (320, 120, 1280.0, 800.0));
}

#[test]
fn the_frame_is_centred_with_the_content() {
    // outer 1296×839 in 1920×1040: (1920 − 1296) / 2 = 312, (1040 − 839) / 2 = 100
    assert_eq!(window(place(&area(0, 0, 1920, 1040, 1.0), FRAME_100)), (312, 100, 1280.0, 800.0));
}

#[test]
fn a_scaled_screen_is_measured_in_its_own_logical_pixels() {
    // 1920×1040 at 150 % is 1280×693.3 logical, minus 24 on each side
    let (x, y, width, height) = window(place(&area(0, 0, 1920, 1040, 1.5), Frame::default()));
    assert!(close(width, 1232.0) && close(height, 645.33), "{width}×{height}");
    assert_eq!((x, y), (36, 36), "centred: (1920 − 1848) / 2, (1040 − 968) / 2");
}

#[test]
fn a_small_laptop_screen_opens_maximised_within_the_screen() {
    // 1366×728 at 125 % is 1092.8×582.4 logical: under 600 high once the margins are off
    let placement = place(&area(0, 0, 1366, 728, 1.25), Frame::default());
    let Placement::Maximized { min_width, min_height } = placement else { panic!("{placement:?}") };
    assert_eq!(min_width, MIN_WIDTH, "the width still holds the usual minimum");
    assert!(close(min_height, 582.4), "{min_height}");
}

#[test]
fn a_maximised_minimum_leaves_room_for_the_title_bar() {
    let frame = Frame { left: 10, top: 39, right: 10, bottom: 10 };
    let placement = place(&area(0, 0, 1366, 728, 1.25), frame);
    let Placement::Maximized { min_height, .. } = placement else { panic!("{placement:?}") };
    assert!(close(min_height, (728.0 - 49.0) / 1.25), "{min_height}");
}

#[test]
fn a_720p_screen_keeps_the_margins() {
    assert_eq!(window(place(&area(0, 0, 1280, 720, 1.0), Frame::default())), (24, 24, 1232.0, 672.0));
}

#[test]
fn a_secondary_screen_left_of_the_primary_keeps_negative_coordinates() {
    let (x, y, ..) = window(place(&area(-1920, 0, 1920, 1080, 1.0), Frame::default()));
    assert_eq!((x, y), (-1920 + 320, 140));
}

#[test]
fn a_top_taskbar_pushes_the_window_down() {
    let (_, y, _, height) = window(place(&area(0, 40, 1280, 680, 1.0), Frame::default()));
    assert!(y >= 40, "y {y} is under the taskbar");
    assert_eq!(height, 632.0);
}

#[test]
fn exactly_the_minimum_plus_margins_is_a_window_not_maximised() {
    let fits = (MIN_WIDTH + 48.0) as u32;
    let tall = (MIN_HEIGHT + 48.0) as u32;
    assert_eq!(window(place(&area(0, 0, fits, tall, 1.0), Frame::default())), (24, 24, MIN_WIDTH, MIN_HEIGHT));
    assert!(matches!(place(&area(0, 0, fits - 1, tall, 1.0), Frame::default()), Placement::Maximized { .. }));
    assert!(matches!(place(&area(0, 0, fits, tall - 1, 1.0), Frame::default()), Placement::Maximized { .. }));
}

proptest! {
    #[test]
    fn the_window_always_fits_its_screen(
        width in 800u32..=3840,
        height in 600u32..=2160,
        left in -3840i32..=3840,
        top in -1000i32..=1000,
        scale in prop::sample::select(vec![1.0f32, 1.25, 1.5, 1.75, 2.0]),
        frame in prop::sample::select(vec![Frame::default(), FRAME_100]),
    ) {
        let work = area(left, top, width, height, scale);
        let (logical_width, logical_height) = (width as f32 / scale, height as f32 / scale);
        let minimum_fits = logical_width - 48.0 >= MIN_WIDTH && logical_height - 48.0 >= MIN_HEIGHT;
        match place(&work, frame) {
            Placement::Window { x, y, width: w, height: h, maximized } => {
                prop_assert!(!maximized);
                prop_assert!(minimum_fits, "a window where the minimum doesn't fit");
                prop_assert!(w <= PREFERRED_WIDTH && h <= PREFERRED_HEIGHT);
                prop_assert!(w >= MIN_WIDTH && h >= MIN_HEIGHT);
                let outer_width = (w * scale).round() as i32 + (frame.left + frame.right) as i32;
                let outer_height = (h * scale).round() as i32 + (frame.top + frame.bottom) as i32;
                prop_assert!(x >= left && y >= top, "above or left of the work area");
                prop_assert!(x + outer_width <= left + width as i32, "past the right edge");
                prop_assert!(y + outer_height <= top + height as i32, "past the bottom edge");
            }
            Placement::Maximized { min_width, min_height } => {
                prop_assert!(!minimum_fits, "maximised although the minimum fits");
                prop_assert!(min_width <= MIN_WIDTH && min_height <= MIN_HEIGHT);
                prop_assert!(min_width * scale + (frame.left + frame.right) as f32 <= width as f32 + 0.5);
                prop_assert!(min_height * scale + (frame.top + frame.bottom) as f32 <= height as f32 + 0.5);
            }
        }
    }
}

#[test]
fn the_window_takes_the_placed_size() {
    let ui = Ui::with_database();
    placement::apply_with(&ui.session.window, Ok((area(0, 0, 1280, 720, 1.0), Frame::default())));
    assert_eq!(ui.session.window.window().size(), slint::PhysicalSize::new(1232, 672));
    assert!(!ui.session.window.window().is_maximized());
}

#[test]
fn a_screen_too_small_maximises_the_window_and_lowers_its_minimum() {
    let ui = Ui::with_database();
    placement::apply_with(&ui.session.window, Ok((area(0, 0, 1366, 728, 1.25), Frame::default())));
    assert!(ui.session.window.window().is_maximized());
    assert_eq!(ui.session.window.get_fit_min_width(), MIN_WIDTH);
    assert!(close(ui.session.window.get_fit_min_height(), 582.4));
}

#[test]
fn the_primary_screen_is_read_wherever_the_mouse_is() {
    let (primary, frame) = placement::work_area_at(placement::PRIMARY_SCREEN).expect("the primary screen is read");
    assert!(primary.width > 0 && primary.height > 0 && primary.scale >= 1.0, "{primary:?}");
    assert!(frame.top >= frame.bottom, "the title bar is part of the frame: {frame:?}");
    // a point on no monitor falls back to the primary one, as the tests' driver relies on
    assert_eq!(placement::work_area_at((-1_000_000, -1_000_000)), Ok((primary, frame)));
}

fn saved(x: i32, y: i32, width: u32, height: u32) -> WindowPlace {
    WindowPlace { x, y, width, height, maximized: false }
}

const FULL_HD: (u32, u32) = (1920, 1040);

#[test]
fn a_saved_place_that_fits_is_kept() {
    let screen = area(0, 0, FULL_HD.0, FULL_HD.1, 1.0);
    assert_eq!(
        restore(&saved(100, 50, 1100, 700), &screen, FRAME_100),
        Some(Placement::Window { x: 100, y: 50, width: 1100.0, height: 700.0, maximized: false })
    );
}

#[test]
fn a_saved_place_past_the_edges_is_pulled_back_onto_the_screen() {
    let screen = area(0, 0, FULL_HD.0, FULL_HD.1, 1.0);
    // the outer window is 1116×739 with the frame
    let restored = window(restore(&saved(1800, 900, 1100, 700), &screen, FRAME_100).unwrap());
    assert_eq!(restored, (1920 - 1116, 1040 - 739, 1100.0, 700.0));
    let restored = window(restore(&saved(-500, -300, 1100, 700), &screen, FRAME_100).unwrap());
    assert_eq!((restored.0, restored.1), (0, 0), "never left of or above the work area");
}

#[test]
fn a_saved_size_is_kept_between_the_minimum_and_the_screen() {
    let screen = area(0, 0, 1366, 728, 1.0);
    let restored = window(restore(&saved(0, 0, 4000, 3000), &screen, FRAME_100).unwrap());
    assert_eq!(restored, (0, 0, 1366.0 - 16.0, 728.0 - 39.0), "no larger than the work area");
    let restored = window(restore(&saved(10, 10, 200, 100), &screen, FRAME_100).unwrap());
    assert_eq!((restored.2, restored.3), (MIN_WIDTH, MIN_HEIGHT), "no smaller than the minimum");
}

#[test]
fn a_saved_place_on_a_second_screen_stays_there() {
    // a screen right of the primary one, at 150 %
    let screen = area(1920, -200, 2560, 1400, 1.5);
    let restored = restore(&saved(2100, -100, 1200, 750), &screen, FRAME_100).unwrap();
    assert_eq!(restored, Placement::Window { x: 2100, y: -100, width: 1200.0, height: 750.0, maximized: false });
}

#[test]
fn a_maximised_window_opens_maximised_returning_to_its_place() {
    let screen = area(0, 0, FULL_HD.0, FULL_HD.1, 1.0);
    let place = WindowPlace { maximized: true, ..saved(200, 100, 1100, 700) };
    assert_eq!(
        restore(&place, &screen, FRAME_100),
        Some(Placement::Window { x: 200, y: 100, width: 1100.0, height: 700.0, maximized: true })
    );
}

#[test]
fn a_screen_too_small_for_the_minimum_places_the_window_afresh() {
    assert_eq!(restore(&saved(0, 0, 1100, 700), &area(0, 0, 1366, 600, 1.0), FRAME_100), None);
    assert_eq!(restore(&saved(0, 0, 1100, 700), &area(0, 0, 1366, 728, 1.25), FRAME_100), None);
    assert_eq!(
        restore(&saved(0, 0, 1100, 700), &area(0, 0, 900, 1440, 1.0), FRAME_100),
        None,
        "tall enough, but too narrow"
    );
}

proptest! {
    #[test]
    fn a_restored_window_always_lies_on_its_screen(
        x in -10_000i32..=10_000,
        y in -10_000i32..=10_000,
        saved_width in 0u32..=8000,
        saved_height in 0u32..=8000,
        width in 1100u32..=3840,
        height in 700u32..=2160,
        left in -3840i32..=3840,
        top in -1000i32..=1000,
        frame in prop::sample::select(vec![Frame::default(), FRAME_100]),
    ) {
        let work = area(left, top, width, height, 1.0);
        let Some(Placement::Window { x: at_x, y: at_y, width: w, height: h, .. }) =
            restore(&saved(x, y, saved_width, saved_height), &work, frame)
        else {
            return Err(TestCaseError::fail("a screen this large holds the minimum"));
        };
        prop_assert!(w >= MIN_WIDTH && h >= MIN_HEIGHT);
        let outer_width = w as i32 + (frame.left + frame.right) as i32;
        let outer_height = h as i32 + (frame.top + frame.bottom) as i32;
        prop_assert!(at_x >= left && at_y >= top, "above or left of the work area");
        prop_assert!(at_x + outer_width <= left + width as i32, "past the right edge");
        prop_assert!(at_y + outer_height <= top + height as i32, "past the bottom edge");
    }
}

#[test]
fn what_is_kept_as_the_window_closes() {
    let before = Some(saved(100, 50, 1100, 700));
    let normal = Shown::Normal { x: 300, y: 200, width: 1199.6, height: 750.2 };
    assert_eq!(remembered(normal, before), Some(saved(300, 200, 1200, 750)));
    // maximised: the place it returns to is the one kept before
    let maximized = Shown::Maximized { x: -8, y: -8 };
    assert_eq!(remembered(maximized, before), Some(WindowPlace { maximized: true, ..saved(100, 50, 1100, 700) }));
    // with none kept before, the preferred size on the screen it was maximised on
    assert_eq!(
        remembered(maximized, None),
        Some(WindowPlace { maximized: true, ..saved(-8, -8, PREFERRED_WIDTH as u32, PREFERRED_HEIGHT as u32) })
    );
    // minimised (closed from the taskbar): the last place stays
    assert_eq!(remembered(Shown::Minimized, before), None);
}

#[test]
fn a_saved_place_off_every_screen_is_found_on_none() {
    assert_eq!(placement::work_area_of(&saved(-1_000_000, -1_000_000, 1100, 700)), Ok(None));
    let (primary, frame) = placement::work_area_at(placement::PRIMARY_SCREEN).unwrap();
    assert_eq!(
        placement::work_area_of(&saved(primary.left + 10, primary.top + 10, 1100, 700)),
        Ok(Some((primary, frame)))
    );
}

/// Closes the window as its close button does.
fn close_window(ui: &Ui) {
    ui.session.window.window().dispatch_event(slint::platform::WindowEvent::CloseRequested);
}

#[test]
fn closing_the_window_keeps_its_place_for_the_next_start() {
    let ui = Ui::with_database();
    // a small screen (a CI runner's) opens the window maximised, and only a restored one takes a size
    ui.session.window.window().set_maximized(false);
    ui.session.window.window().set_size(slint::LogicalSize::new(1111.0, 666.0));
    close_window(&ui);
    let kept = ui.session.state.borrow().preferences().window.expect("the place is kept");
    assert_eq!((kept.width, kept.height, kept.maximized), (1111, 666, false));
    let ui = ui.restart();
    assert_eq!(ui.session.state.borrow().preferences().window, Some(kept), "and read again at the next start");
}

#[test]
fn closing_before_the_first_run_is_done_keeps_nothing() {
    let ui = Ui::open(common::Fixture::new());
    close_window(&ui);
    assert!(!ui.fixture.store().path().exists(), "no settings file is written for it");
}

#[test]
fn a_window_saved_on_a_screen_that_is_gone_is_placed_afresh() {
    let ui = Ui::with_database();
    let window = &ui.session.window;
    placement::apply(window, Some(&saved(-30_000, -30_000, 1100, 700)));
    let position = window.window().position();
    assert_ne!((position.x, position.y), (-30_000, -30_000), "not off every screen");
}

#[test]
fn a_maximised_window_keeps_its_normal_place_and_a_minimised_one_its_last() {
    let ui = Ui::with_database();
    let window = ui.session.window.window();
    window.set_size(slint::LogicalSize::new(1111.0, 666.0));
    placement::remember(&ui.session.window, &ui.session.state);
    let kept = || ui.session.state.borrow().preferences().window.expect("the place is kept");
    let normal = kept();
    window.set_maximized(true);
    placement::remember(&ui.session.window, &ui.session.state);
    assert_eq!(kept(), WindowPlace { maximized: true, ..normal });
    window.set_minimized(true);
    placement::remember(&ui.session.window, &ui.session.state);
    assert_eq!(kept(), WindowPlace { maximized: true, ..normal }, "minimised: the last place stays");
}

#[test]
fn a_place_that_did_not_change_is_not_written_again() {
    let ui = Ui::with_database();
    let settings = ui.fixture.store().path().to_path_buf();
    let mut state = ui.session.state.borrow_mut();
    state.set_window_place(saved(10, 20, 1100, 700)).unwrap();
    std::fs::remove_file(&settings).unwrap();
    state.set_window_place(saved(10, 20, 1100, 700)).unwrap();
    assert!(!settings.exists(), "nothing was written");
    state.set_window_place(saved(11, 20, 1100, 700)).unwrap();
    assert!(settings.exists(), "a new place is");
}

#[test]
fn a_place_that_cannot_be_saved_does_not_stop_the_window_closing() {
    let ui = Ui::with_database();
    let config = ui.fixture.path("config");
    std::fs::remove_dir_all(&config).unwrap();
    std::fs::write(&config, b"in the way").unwrap();
    ui.session.window.window().set_size(slint::LogicalSize::new(1001.0, 701.0));
    close_window(&ui);
    assert!(!ui.session.window.window().is_visible());
}
