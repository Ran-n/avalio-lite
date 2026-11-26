// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:16.033581
//+ Revised: 	2026/11/25 11:43:03.160244
// -----------------------------------------------------------------------------

//! Pupils' pictures: turning a photo into the small square the database keeps, storing it, and reading
//! it back as the pixels of a round avatar. What a photo carries besides its pixels (where it was
//! taken, with what) is never kept.

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader, Limits, RgbaImage};
use rusqlite::{Connection, OptionalExtension, params};

use super::update_one;
use crate::error::{AppError, Result};

/// The side of the stored square, in pixels: enough for the largest avatar at 200 % scaling.
pub const PICTURE_SIDE: u32 = 256;
/// The largest photo file read; a phone's photos are a tenth of it.
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;
const MAX_PHOTO_SIDE: u32 = 16_000;
const MAX_PHOTO_MEMORY: u64 = 512 * 1024 * 1024;
const JPEG_QUALITY: u8 = 85;

/// Reads the photo at `path` and makes the picture to store.
pub fn from_file(path: &Path) -> Result<Vec<u8>> {
    let unreadable = |reason: String| AppError::PictureRead { path: path.to_path_buf(), reason };
    let size = fs::metadata(path).map_err(|error| unreadable(error.to_string()))?.len();
    if size > MAX_FILE_BYTES {
        return Err(unreadable(format!("{size} bytes is more than {MAX_FILE_BYTES}")));
    }
    let bytes = fs::read(path).map_err(|error| unreadable(error.to_string()))?;
    from_bytes(&bytes).map_err(|error| unreadable(error.to_string()))
}

/// The middle square of a photo, the right way up, as a small JPEG with nothing but its pixels.
fn from_bytes(photo: &[u8]) -> image::ImageResult<Vec<u8>> {
    let square = middle_square(&upright(photo)?);
    let flat = on_white(&square.resize_exact(PICTURE_SIDE, PICTURE_SIDE, FilterType::Triangle).to_rgba8());
    let mut picture = Vec::new();
    JpegEncoder::new_with_quality(&mut picture, JPEG_QUALITY).encode_image(&flat)?;
    Ok(picture)
}

/// The photo turned as the camera noted it was held.
fn upright(photo: &[u8]) -> image::ImageResult<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(photo)).with_guessed_format()?;
    reader.limits(limits());
    let mut decoder = reader.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image)
}

/// What a photo may take to open, so a small file that unpacks to a huge one is refused.
fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_PHOTO_SIDE);
    limits.max_image_height = Some(MAX_PHOTO_SIDE);
    limits.max_alloc = Some(MAX_PHOTO_MEMORY);
    limits
}

fn middle_square(image: &DynamicImage) -> DynamicImage {
    let (width, height) = (image.width(), image.height());
    let side = width.min(height);
    image.crop_imm((width - side) / 2, (height - side) / 2, side, side)
}

/// A JPEG has no transparency, so what is see-through becomes white.
fn on_white(image: &RgbaImage) -> DynamicImage {
    let flat = image::RgbImage::from_fn(image.width(), image.height(), |x, y| {
        let [red, green, blue, alpha] = image.get_pixel(x, y).0;
        let over = |channel: u8| {
            let mixed = u32::from(channel) * u32::from(alpha) + 255 * (255 - u32::from(alpha));
            u8::try_from(mixed / 255).unwrap_or(u8::MAX)
        };
        image::Rgb([over(red), over(green), over(blue)])
    });
    DynamicImage::ImageRgb8(flat)
}

/// A stored picture as a round avatar of `side` pixels, transparent outside the circle.
pub fn avatar(picture: &[u8], side: u32) -> image::ImageResult<RgbaImage> {
    let image = upright(picture)?;
    let mut avatar = image.resize_exact(side, side, FilterType::Triangle).to_rgba8();
    let radius = side as f32 / 2.0;
    for (x, y, pixel) in avatar.enumerate_pixels_mut() {
        let (dx, dy) = (x as f32 + 0.5 - radius, y as f32 + 0.5 - radius);
        // 1 inside the circle, 0 outside, and in between along its edge
        let cover = (radius - dx.hypot(dy) + 0.5).clamp(0.0, 1.0);
        pixel.0[3] = (cover * 255.0).round() as u8;
    }
    Ok(avatar)
}

/// Stores the pupil's picture in place of any they have; `NotEditable` when there is no such pupil.
pub fn set_picture(connection: &Connection, pupil_id: i64, picture: &[u8]) -> Result<()> {
    update_one(
        connection,
        "INSERT INTO pupil_pictures (pupil_id, image) SELECT id, ?2 FROM pupils WHERE id = ?1
         ON CONFLICT (pupil_id) DO UPDATE SET image = excluded.image",
        params![pupil_id, picture],
    )?;
    log::info!("set the picture of pupil {pupil_id}");
    Ok(())
}

/// Deletes the pupil's picture; `NotEditable` when they have none.
pub fn remove_picture(connection: &Connection, pupil_id: i64) -> Result<()> {
    update_one(connection, "DELETE FROM pupil_pictures WHERE pupil_id = ?1", [pupil_id])?;
    log::info!("removed the picture of pupil {pupil_id}");
    Ok(())
}

pub fn picture(connection: &Connection, pupil_id: i64) -> Result<Option<Vec<u8>>> {
    let sql = "SELECT image FROM pupil_pictures WHERE pupil_id = ?1";
    Ok(connection.query_row(sql, [pupil_id], |row| row.get(0)).optional()?)
}

/// The pictures of a class's pupils, by pupil id.
pub fn class_pictures(connection: &Connection, class_id: i64) -> Result<HashMap<i64, Vec<u8>>> {
    let mut statement = connection.prepare(
        "SELECT p.pupil_id, p.image FROM pupil_pictures p JOIN class_pupils cp ON cp.pupil_id = p.pupil_id
         WHERE cp.class_id = ?1",
    )?;
    let pictures =
        statement.query_map([class_id], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
    Ok(pictures)
}
