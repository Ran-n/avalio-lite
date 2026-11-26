// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:51:22.454734
//+ Revised: 	2026/11/25 11:43:14.470496
// -----------------------------------------------------------------------------

//! Photo files that aren't accepted as a pupil's picture, and what is said of them.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use avalio_lite::error::AppError;
use avalio_lite::gradebook::pictures::{self, MAX_FILE_BYTES};
use image::codecs::png::PngEncoder;
use image::{ImageEncoder, Rgb, RgbImage};

fn refused(path: &Path) -> AppError {
    let error = pictures::from_file(path).unwrap_err();
    assert!(matches!(&error, AppError::PictureRead { path: named, .. } if named == path), "{error:?}");
    error
}

#[test]
fn files_that_are_not_pictures_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let whole = dir.path().join("whole.png");
    RgbImage::from_fn(300, 300, |x, y| Rgb([x as u8, y as u8, 7])).save(&whole).unwrap();
    let half = fs::read(&whole).unwrap();
    for (name, bytes) in [
        ("empty.jpg", Vec::new()),
        ("text.jpg", b"not a picture".to_vec()),
        ("cut.png", half[..half.len() / 2].to_vec()),
        // a kind the app doesn't read
        ("a.gif", b"GIF89a\x01\0\x01\0\0\0\0;".to_vec()),
    ] {
        let path = dir.path().join(name);
        fs::write(&path, bytes).unwrap();
        refused(&path);
    }
    refused(&dir.path().join("missing.jpg"));
    refused(dir.path());
}

/// A photo's file is often named after the pupil, so what goes to the log leaves it out.
#[test]
fn a_refused_photos_name_never_reaches_the_log_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Ana Pérez.jpg");
    fs::write(&path, b"not a picture").unwrap();
    let error = refused(&path);
    assert!(!error.to_string().contains("Ana"), "{error}");
    assert!(!refused(&dir.path().join("Ana missing.jpg")).to_string().contains("Ana"));
}

#[test]
fn a_file_too_big_is_refused_unread() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("huge.jpg");
    fs::File::create(&path).unwrap().set_len(MAX_FILE_BYTES + 1).unwrap();
    assert!(refused(&path).to_string().contains("is more than"));
}

/// A small file that says it unpacks to a huge photo is refused before it is unpacked.
#[test]
fn a_photo_of_too_many_pixels_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let row = RgbImage::from_pixel(16_001, 1, Rgb([220, 20, 20]));
    let mut bytes = Vec::new();
    PngEncoder::new(Cursor::new(&mut bytes)).write_image(&row, 16_001, 1, image::ExtendedColorType::Rgb8).unwrap();
    let path = dir.path().join("wide.png");
    fs::write(&path, bytes).unwrap();
    refused(&path);
}
