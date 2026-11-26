// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/26 05:25:10.768901
//+ Revised: 	2026/11/26 05:30:38.191383
// -----------------------------------------------------------------------------

//! The README's screenshots follow the reader's light or dark mode, on every forge. GitLab and Codeberg
//! strip `<picture>`, so each screenshot is one SVG (`assets/screenshots/`) holding the desktop tests'
//! light and dark captures of a screen, which shows the right one by itself. A changed baseline fails
//! this test until the SVGs are written again: `AVALIO_UPDATE_GOLDEN=1 cargo test --test readme`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

const BASELINES: &str = "tests/desktop/baselines";
const SCREENSHOTS: &str = "assets/screenshots";

/// A screenshot: the SVG's name, the light and dark captures it holds and their size.
struct Shot {
    name: &'static str,
    light: &'static str,
    dark: &'static str,
    size: (u32, u32),
}

/// A screen captured by `every_look` at the normal size (`tests/desktop/screens.rs`).
macro_rules! normal {
    ($screen:literal) => {
        Shot {
            name: $screen,
            light: concat!($screen, "-light-1280x800@100.png"),
            dark: concat!($screen, "-dark-1280x800@100.png"),
            size: (1280, 800),
        }
    };
}

const SHOTS: [Shot; 8] = [
    normal!("sheet"),
    normal!("column-panel"),
    normal!("sheet-menu"),
    normal!("lock"),
    normal!("setup"),
    normal!("settings-backup"),
    normal!("settings"),
    Shot {
        name: "sheet-gl",
        light: "sheet-gl-1000x600@100.png",
        dark: "sheet-gl-dark-1000x600@100.png",
        size: (1000, 600),
    },
];

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn capture(file: &str) -> Vec<u8> {
    let path = repo(BASELINES).join(file);
    fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn svg_path(name: &str) -> PathBuf {
    repo(SCREENSHOTS).join(format!("{name}.svg"))
}

/// Both captures in one SVG; a style picks the one for the reader's mode (light when it has none).
fn svg(shot: &Shot) -> String {
    let (width, height) = shot.size;
    let image = |class: &str, file: &str| {
        let data = STANDARD.encode(capture(file));
        format!(
            "  <image class=\"{class}\" width=\"{width}\" height=\"{height}\" href=\"data:image/png;base64,{data}\"/>\n"
        )
    };
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n  \
         <style>.dark{{display:none}}@media (prefers-color-scheme:dark){{.light{{display:none}}.dark{{display:inline}}}}</style>\n\
         {}{}</svg>\n",
        image("light", shot.light),
        image("dark", shot.dark)
    )
}

/// The mean of every colour channel.
fn brightness(image: &image::RgbImage) -> u64 {
    let total: u64 = image.pixels().flat_map(|pixel| pixel.0).map(u64::from).sum();
    total / (u64::from(image.width()) * u64::from(image.height()) * 3)
}

/// With `AVALIO_UPDATE_GOLDEN=1`, writes every SVG, once, before any test reads the folder.
fn write_when_updating() {
    static WRITTEN: OnceLock<()> = OnceLock::new();
    WRITTEN.get_or_init(|| {
        if std::env::var_os("AVALIO_UPDATE_GOLDEN").is_some_and(|value| value == "1") {
            fs::create_dir_all(repo(SCREENSHOTS)).unwrap();
            for shot in &SHOTS {
                fs::write(svg_path(shot.name), svg(shot)).unwrap();
            }
        }
    });
}

#[test]
fn each_screenshot_holds_the_current_captures() {
    write_when_updating();
    for shot in &SHOTS {
        let (path, expected) = (svg_path(shot.name), svg(shot));
        let on_disk = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(on_disk == expected, "{} is out of date; write it again with AVALIO_UPDATE_GOLDEN=1", path.display());
    }
}

#[test]
fn each_pair_is_one_size_and_really_light_and_dark() {
    for shot in &SHOTS {
        let [light, dark] =
            [shot.light, shot.dark].map(|file| image::load_from_memory(&capture(file)).unwrap().to_rgb8());
        for (file, image) in [(shot.light, &light), (shot.dark, &dark)] {
            assert_eq!(image.dimensions(), shot.size, "{file}");
        }
        assert!(brightness(&dark) + 50 < brightness(&light), "{} is not clearly darker than {}", shot.dark, shot.light);
    }
}

#[test]
fn the_readme_shows_every_screenshot_and_no_other() {
    let readme = fs::read_to_string(repo("README.md")).unwrap();
    let shown: BTreeSet<&str> =
        readme.split("src=\"assets/screenshots/").skip(1).filter_map(|rest| rest.split(".svg\"").next()).collect();
    assert_eq!(shown, SHOTS.iter().map(|shot| shot.name).collect::<BTreeSet<_>>());
    // a capture shown directly, or <picture>, stays light on GitLab and Codeberg
    for one_mode in [BASELINES, "<picture"] {
        assert!(!readme.contains(one_mode), "the README shows a screenshot with {one_mode}, not its SVG");
    }
    write_when_updating();
    let files: BTreeSet<String> = fs::read_dir(repo(SCREENSHOTS))
        .unwrap()
        .map(|entry| entry.unwrap().path().file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(files, SHOTS.iter().map(|shot| shot.name.to_owned()).collect(), "no left-over SVG");
}
