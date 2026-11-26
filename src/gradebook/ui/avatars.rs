// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:16.443698
//+ Revised: 	2026/11/25 11:43:03.741562
// -----------------------------------------------------------------------------

//! Pupils' pictures as Slint images, and the initials shown where there is none.

use std::collections::HashMap;

use rusqlite::Connection;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::error::Result;
use crate::gradebook::model::Pupil;
use crate::gradebook::pictures;

/// The pixels an avatar is drawn from, at least twice its size on screen so it stays sharp when
/// Windows scales the display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// the start of a sheet's row
    Row,
    /// the pupil view's menu and page
    Card,
    /// the pupil panel
    Panel,
}

impl Size {
    fn pixels(self) -> u32 {
        match self {
            Self::Row => 64,
            Self::Card => 128,
            Self::Panel => pictures::PICTURE_SIDE,
        }
    }
}

/// The avatars of a class's pupils who have a picture, by pupil id.
pub fn class_avatars(connection: &Connection, class_id: i64, size: Size) -> Result<HashMap<i64, Image>> {
    let pictures = pictures::class_pictures(connection, class_id)?;
    Ok(pictures.into_iter().map(|(pupil_id, picture)| (pupil_id, avatar(pupil_id, &picture, size))).collect())
}

/// The pupil's avatar; an empty image when they have no picture.
pub fn pupil_avatar(connection: &Connection, pupil_id: i64, size: Size) -> Result<Image> {
    let picture = pictures::picture(connection, pupil_id)?;
    Ok(picture.map(|picture| avatar(pupil_id, &picture, size)).unwrap_or_default())
}

/// A stored picture that can't be read (a damaged file) leaves the initials, and is logged.
fn avatar(pupil_id: i64, picture: &[u8], size: Size) -> Image {
    match pictures::avatar(picture, size.pixels()) {
        Ok(pixels) => {
            let (width, height) = pixels.dimensions();
            let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(pixels.as_raw(), width, height);
            Image::from_rgba8(buffer)
        }
        Err(error) => {
            log::error!("the stored picture of pupil {pupil_id} can't be shown: {error}");
            Image::default()
        }
    }
}

/// "AP" for Ana Pérez, "A" without a surname.
pub fn initials(pupil: &Pupil) -> String {
    [&pupil.first_name, &pupil.surname]
        .into_iter()
        .filter_map(|name| name.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}
