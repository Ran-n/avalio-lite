// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:27:41.589331
//+ Revised: 	2026/11/24 17:54:11.059150
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

// ---------------------------------------------------------------------------------------------
// Breren's logo in Settings › About: the original, never a simplified mark

const BREREN_LIGHT: &str = include_str!("../assets/breren-logo.svg");
const BREREN_DARK: &str = include_str!("../assets/breren-logo-dark.svg");

/// The drawing without its `<style>`: the paths and circles that make the shape.
fn shapes(svg: &str) -> String {
    let start = svg.find("<style>").expect("a style block");
    let end = svg.find("</style>").expect("a style block") + "</style>".len();
    format!("{}{}", &svg[..start], &svg[end..])
}

#[test]
fn the_breren_logo_is_the_original_with_a_dark_twin_of_the_same_shape() {
    // the original as the site has it: three rampart rings and the huts, with its dark colours
    // behind a media query that Slint doesn't read
    for class in ["class=\"outer\"", "class=\"middle\"", "class=\"inner\"", "class=\"hut\""] {
        assert!(BREREN_LIGHT.contains(class), "the original draws {class}");
    }
    assert!(BREREN_LIGHT.contains("@media (prefers-color-scheme: dark)"), "copied unchanged");
    assert_eq!(shapes(BREREN_DARK), shapes(BREREN_LIGHT), "the dark file only changes the colours");
    for colour in ["#b0a696", "#a39d95", "#928878", "#c9944f"] {
        assert!(BREREN_DARK.contains(colour), "dark colour {colour}");
    }
    for light in ["#33302e", "#4a4542", "#635d59", "#6b4423"] {
        assert!(!BREREN_DARK.contains(light), "no light-background colour {light} in the dark file");
    }
    let card = include_str!("../ui/settings/about-card.slint");
    assert!(card.contains("breren-logo-dark.svg") && card.contains("breren-logo.svg"), "picked per theme");
    assert!(!card.contains("breren-square"), "never the simplified mark");
}

#[test]
fn each_breren_logo_file_draws_in_its_own_colours() {
    // the hut dots: brown on light backgrounds, ochre on dark; Slint's renderer, like this one,
    // ignores the original's media query, which is why the dark file exists
    for (name, hut) in [("breren-logo.svg", [0x6b, 0x44, 0x23]), ("breren-logo-dark.svg", [0xc9, 0x94, 0x4f])] {
        let tree = logo::parse(&logo::asset(name)).unwrap();
        assert_eq!(tree.size().width(), tree.size().height(), "{name} is square");
        let pixels = logo::render(&tree, 64, None).unwrap();
        let near = |pixel: &[u8]| pixel[3] == 255 && (0..3).all(|at| pixel[at].abs_diff(hut[at]) <= 12);
        assert!(pixels.as_chunks::<4>().0.iter().any(|pixel| near(pixel)), "{name} draws its huts in {hut:02x?}");
    }
}
