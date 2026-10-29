// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:27:41.714987
//+ Revised: 	2026/10/29 02:27:41.922359
// -----------------------------------------------------------------------------

//! Renders the logo SVGs in `assets/`. Shared by `examples/render_logo.rs`, which writes the icon
//! files from them, and `tests/logo.rs`, which checks those files are up to date.

use std::fs;
use std::path::{Path, PathBuf};

use resvg::{tiny_skia, usvg};

/// The image sizes in `assets/logo.ico`; Windows picks the one it needs for each place.
pub const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];
/// The tile colour of `logo.svg` (the light theme's accent).
pub const TILE: [u8; 3] = [0x2f, 0x54, 0xd6];

pub fn asset(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets").join(name)
}

pub fn parse(path: &Path) -> Result<usvg::Tree, String> {
    let data = fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    usvg::Tree::from_data(&data, &usvg::Options::default())
        .map_err(|error| format!("{} is not a valid SVG: {error}", path.display()))
}

/// The SVG drawn into a `size` × `size` square over `background` (transparent when `None`),
/// as straight (not premultiplied) RGBA bytes.
pub fn render(tree: &usvg::Tree, size: u32, background: Option<[u8; 3]>) -> Result<Vec<u8>, String> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).ok_or_else(|| format!("can't render at {size} px"))?;
    if let Some([red, green, blue]) = background {
        pixmap.fill(tiny_skia::Color::from_rgba8(red, green, blue, 255));
    }
    let scale = size as f32 / tree.size().width();
    resvg::render(tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    Ok(pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let straight = pixel.demultiply();
            [straight.red(), straight.green(), straight.blue(), straight.alpha()]
        })
        .collect())
}
