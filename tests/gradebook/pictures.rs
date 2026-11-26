// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:17.744513
//+ Revised: 	2026/11/25 11:43:14.191519
// -----------------------------------------------------------------------------

//! Pupils' pictures: what a photo file becomes, and the round avatars drawn from it.

use std::fs;
use std::path::{Path, PathBuf};

use crate::support::open;
use avalio_lite::gradebook::pictures::{self, PICTURE_SIDE};
use avalio_lite::gradebook::{classes, pupils};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageEncoder, ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use proptest::prelude::*;

const RED: Rgb<u8> = Rgb([220, 20, 20]);
const BLUE: Rgb<u8> = Rgb([20, 20, 220]);

/// A photo red on its left half and blue on its right.
fn photo(width: u32, height: u32) -> RgbImage {
    RgbImage::from_fn(width, height, |x, _| if x < width / 2 { RED } else { BLUE })
}

fn saved(folder: &Path, name: &str, image: &DynamicImage, format: ImageFormat) -> PathBuf {
    let path = folder.join(name);
    image.save_with_format(&path, format).unwrap();
    path
}

fn decoded(picture: &[u8]) -> RgbImage {
    image::load_from_memory_with_format(picture, ImageFormat::Jpeg).unwrap().to_rgb8()
}

fn is_close(pixel: &Rgb<u8>, wanted: Rgb<u8>) -> bool {
    pixel.0.iter().zip(wanted.0).all(|(got, want)| got.abs_diff(want) < 40)
}

#[test]
fn every_kind_of_photo_becomes_a_small_square_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    let image = DynamicImage::ImageRgb8(photo(900, 600));
    for (name, format) in [("a.jpg", ImageFormat::Jpeg), ("a.png", ImageFormat::Png), ("a.webp", ImageFormat::WebP)] {
        let picture = decoded(&pictures::from_file(&saved(dir.path(), name, &image, format)).unwrap());
        assert_eq!(picture.dimensions(), (PICTURE_SIDE, PICTURE_SIDE), "{name}");
        assert!(is_close(picture.get_pixel(20, 128), RED) && is_close(picture.get_pixel(236, 128), BLUE), "{name}");
    }
}

/// The kind is read from the file, not from its name.
#[test]
fn a_photo_with_the_wrong_extension_still_opens() {
    let dir = tempfile::tempdir().unwrap();
    let image = DynamicImage::ImageRgb8(photo(300, 300));
    let path = saved(dir.path(), "really-a-png.jpg", &image, ImageFormat::Png);
    assert!(pictures::from_file(&path).is_ok());
}

#[test]
fn the_middle_of_the_photo_is_kept() {
    let dir = tempfile::tempdir().unwrap();
    // a wide photo, all green but for a red band in the middle third
    let wide = RgbImage::from_fn(900, 300, |x, _| if (300..600).contains(&x) { RED } else { Rgb([0, 200, 0]) });
    let path = saved(dir.path(), "wide.png", &DynamicImage::ImageRgb8(wide), ImageFormat::Png);
    let picture = decoded(&pictures::from_file(&path).unwrap());
    for corner in [(3, 3), (252, 3), (3, 252), (252, 252), (128, 128)] {
        assert!(is_close(picture.get_pixel(corner.0, corner.1), RED), "{corner:?}");
    }
}

/// A phone held upright stores the photo on its side and notes how it was held.
#[test]
fn a_photo_is_turned_as_the_camera_was_held() {
    let dir = tempfile::tempdir().unwrap();
    // a minimal EXIF block: little-endian TIFF with one entry, orientation 6 (turn 90° clockwise)
    let exif =
        [b"II*\0\x08\0\0\0\x01\0".as_slice(), b"\x12\x01\x03\0\x01\0\0\0\x06\0\0\0".as_slice(), b"\0\0\0\0".as_slice()]
            .concat();
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 95);
    encoder.set_exif_metadata(exif).unwrap();
    encoder.encode_image(&DynamicImage::ImageRgb8(photo(400, 400))).unwrap();
    let path = dir.path().join("phone.jpg");
    fs::write(&path, &bytes).unwrap();
    let picture = pictures::from_file(&path).unwrap();
    let shown = decoded(&picture);
    assert!(is_close(shown.get_pixel(128, 20), RED), "the left half is now the top");
    assert!(is_close(shown.get_pixel(128, 236), BLUE));
    assert!(!picture.windows(4).any(|bytes| bytes == b"Exif"), "what the camera noted isn't kept");
}

#[test]
fn what_is_see_through_becomes_white() {
    let dir = tempfile::tempdir().unwrap();
    let clear = RgbaImage::from_pixel(300, 300, Rgba([0, 0, 0, 0]));
    let path = saved(dir.path(), "clear.png", &DynamicImage::ImageRgba8(clear), ImageFormat::Png);
    let picture = decoded(&pictures::from_file(&path).unwrap());
    assert!(is_close(picture.get_pixel(128, 128), Rgb([255, 255, 255])));
}

#[test]
fn a_tiny_photo_is_enlarged() {
    let dir = tempfile::tempdir().unwrap();
    let path = saved(dir.path(), "tiny.png", &DynamicImage::ImageRgb8(photo(1, 3)), ImageFormat::Png);
    assert_eq!(decoded(&pictures::from_file(&path).unwrap()).dimensions(), (PICTURE_SIDE, PICTURE_SIDE));
}

#[test]
fn an_avatar_is_round() {
    let dir = tempfile::tempdir().unwrap();
    let path = saved(dir.path(), "a.png", &DynamicImage::ImageRgb8(photo(300, 300)), ImageFormat::Png);
    let picture = pictures::from_file(&path).unwrap();
    for side in [64, 128, 256] {
        let avatar = pictures::avatar(&picture, side).unwrap();
        assert_eq!(avatar.dimensions(), (side, side));
        let alpha = |x: u32, y: u32| avatar.get_pixel(x, y).0[3];
        assert_eq!((alpha(0, 0), alpha(side - 1, 0), alpha(0, side - 1), alpha(side - 1, side - 1)), (0, 0, 0, 0));
        assert_eq!((alpha(side / 2, side / 2), alpha(side / 2, 1), alpha(1, side / 2)), (255, 255, 255), "{side}");
    }
}

#[test]
fn a_damaged_stored_picture_is_an_error_not_a_crash() {
    assert!(pictures::avatar(b"damaged", 64).is_err());
    assert!(pictures::avatar(&[], 64).is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// Whatever its shape, a photo becomes a square the database takes.
    #[test]
    fn any_photo_becomes_a_picture_the_database_takes(width in 1u32..400, height in 1u32..400, seed: u8) {
        let dir = tempfile::tempdir().unwrap();
        let noise = RgbImage::from_fn(width, height, |x, y| {
            let value = (x.wrapping_mul(31) ^ y.wrapping_mul(17)).wrapping_mul(u32::from(seed) | 1);
            Rgb([value as u8, (value >> 3) as u8, (value >> 5) as u8])
        });
        let path = saved(dir.path(), "any.png", &DynamicImage::ImageRgb8(noise), ImageFormat::Png);
        let picture = pictures::from_file(&path).unwrap();
        prop_assert_eq!(decoded(&picture).dimensions(), (PICTURE_SIDE, PICTURE_SIDE));
        let (_dir, connection) = open();
        let class = classes::create_class(&connection, "1A").unwrap();
        let ana = pupils::add_new_pupil(&connection, class, "Ana", "").unwrap();
        pictures::set_picture(&connection, ana, &picture).unwrap();
        prop_assert_eq!(pictures::picture(&connection, ana).unwrap(), Some(picture));
    }

    /// No bytes make the reader panic.
    #[test]
    fn no_bytes_make_the_reader_panic(bytes in prop::collection::vec(any::<u8>(), 0..300)) {
        let _refused_or_not = pictures::avatar(&bytes, 64);
    }
}
