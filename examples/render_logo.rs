// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:27:41.465293
//+ Revised: 	2026/10/29 02:27:41.784142
// -----------------------------------------------------------------------------

//! Turns the logo SVGs into the files Windows and the README use, and previews to look at.
//! Usage: cargo run --example render_logo -- <preview folder>
//! Writes assets/logo.ico (16 to 256 px) and assets/logo-256.png from assets/logo.svg, and into
//! the preview folder both variants at 256, 32 and 16 px on white and on the dark canvas.

// shared with tests/logo.rs, which also uses what this example doesn't
#[allow(dead_code)]
#[path = "../tests/common/logo.rs"]
mod logo;

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const PREVIEW_SIZES: [u32; 3] = [256, 32, 16];
const BACKGROUNDS: [(&str, [u8; 3]); 2] = [("on-white", [0xff, 0xff, 0xff]), ("on-dark", [0x13, 0x16, 0x1c])];

fn main() -> ExitCode {
    let Some(previews) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: cargo run --example render_logo -- <preview folder>");
        return ExitCode::FAILURE;
    };
    match run(&previews) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("render_logo: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(previews: &Path) -> Result<(), String> {
    let light = logo::parse(&logo::asset("logo.svg"))?;
    write_ico(&light, &logo::asset("logo.ico"))?;
    write_png(&logo::render(&light, 256, None)?, 256, &logo::asset("logo-256.png"))?;
    fs::create_dir_all(previews).map_err(|error| format!("could not create {}: {error}", previews.display()))?;
    for name in ["logo", "logo-dark"] {
        let tree = logo::parse(&logo::asset(&format!("{name}.svg")))?;
        for size in PREVIEW_SIZES {
            for (label, colour) in BACKGROUNDS {
                let path = previews.join(format!("{name}-{size}-{label}.png"));
                write_png(&logo::render(&tree, size, Some(colour))?, size, &path)?;
            }
        }
    }
    println!("wrote assets/logo.ico, assets/logo-256.png and previews in {}", previews.display());
    Ok(())
}

fn write_ico(tree: &resvg::usvg::Tree, path: &Path) -> Result<(), String> {
    let mut icon = ico::IconDir::new(ico::ResourceType::Icon);
    for size in logo::ICO_SIZES {
        let image = ico::IconImage::from_rgba_data(size, size, logo::render(tree, size, None)?);
        icon.add_entry(ico::IconDirEntry::encode(&image).map_err(|error| format!("{size} px icon: {error}"))?);
    }
    let file = File::create(path).map_err(|error| format!("could not create {}: {error}", path.display()))?;
    icon.write(BufWriter::new(file)).map_err(|error| format!("could not write {}: {error}", path.display()))
}

fn write_png(rgba: &[u8], size: u32, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|error| format!("could not create {}: {error}", path.display()))?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let failed = |error: png::EncodingError| format!("could not write {}: {error}", path.display());
    encoder.write_header().map_err(failed)?.write_image_data(rgba).map_err(failed)
}
