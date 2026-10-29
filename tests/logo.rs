// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:27:41.589331
//+ Revised: 	2026/10/29 02:27:41.853806
// -----------------------------------------------------------------------------

//! The logo files: valid, square, with every icon size Windows uses, generated from the current
//! SVG, and readable at the smallest size.

mod common;

use std::fs::File;

use common::logo;

fn decoded_ico() -> Vec<ico::IconImage> {
    let icon = ico::IconDir::read(File::open(logo::asset("logo.ico")).unwrap()).unwrap();
    icon.entries().iter().map(|entry| entry.decode().unwrap()).collect()
}

#[test]
fn both_logo_variants_parse_and_are_square() {
    for name in ["logo.svg", "logo-dark.svg"] {
        let tree = logo::parse(&logo::asset(name)).unwrap();
        let size = tree.size();
        assert_eq!((size.width(), size.height()), (64.0, 64.0), "{name}");
    }
}

#[test]
fn ico_has_every_windows_size() {
    let mut sizes: Vec<u32> = decoded_ico().iter().map(|image| image.width()).collect();
    sizes.sort_unstable();
    assert_eq!(sizes, logo::ICO_SIZES);
    assert!(decoded_ico().iter().all(|image| image.width() == image.height()));
}

/// Fails when the SVG was edited but the icon not regenerated (`cargo run --example render_logo`).
#[test]
fn ico_is_up_to_date_with_the_svg() {
    let expected = logo::render(&logo::parse(&logo::asset("logo.svg")).unwrap(), 32, None).unwrap();
    let image = decoded_ico().into_iter().find(|image| image.width() == 32).unwrap();
    let worst = expected.iter().zip(image.rgba_data()).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
    assert!(worst <= 2, "the 32 px icon differs from the SVG by up to {worst}; regenerate it");
}

/// At taskbar size the tick's corner is still white on the blue tile, not blurred into it.
#[test]
fn logo_stays_legible_at_16px() {
    let rgba = logo::render(&logo::parse(&logo::asset("logo.svg")).unwrap(), 16, None).unwrap();
    let pixel = |x: usize, y: usize| {
        let at = (y * 16 + x) * 4;
        [rgba[at], rgba[at + 1], rgba[at + 2]]
    };
    let distance = |a: [u8; 3], b: [u8; 3]| a.iter().zip(b).map(|(a, b)| i32::from(a.abs_diff(b))).sum::<i32>();
    let corner = pixel(6, 10);
    assert!(distance(corner, [255, 255, 255]) < distance(corner, logo::TILE), "tick corner is {corner:?}");
}
