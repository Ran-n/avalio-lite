// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 21:48:11.540835
//+ Revised: 	2026/11/12 14:39:57.169503
// -----------------------------------------------------------------------------

//! Where the window opens (`app::placement::place`): the size and place rules on screens of every
//! size and scale, apart from any real screen.

mod common;

use avalio_lite::app::placement::{
    self, Frame, MIN_HEIGHT, MIN_WIDTH, PREFERRED_HEIGHT, PREFERRED_WIDTH, Placement, WorkArea, place,
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

fn area(left: i32, top: i32, width: u32, height: u32, scale: f32) -> WorkArea {
    WorkArea { left, top, width, height, scale }
}

fn window(placement: Placement) -> (i32, i32, f32, f32) {
    match placement {
        Placement::Window { x, y, width, height } => (x, y, width, height),
        Placement::Maximized { .. } => panic!("expected a window, got {placement:?}"),
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
            Placement::Window { x, y, width: w, height: h } => {
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
