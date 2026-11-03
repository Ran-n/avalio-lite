// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.790620
//+ Revised: 	2026/11/03 14:00:40.036558
// -----------------------------------------------------------------------------

//! Captured screens: finding controls by their exact theme colours, and comparing whole screens
//! with reviewed baselines.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// `#rrggbb`, as written in `ui/theme.slint`.
    pub fn hex(text: &str) -> Self {
        let value = u32::from_str_radix(text.trim_start_matches('#'), 16).unwrap();
        Self((value >> 16) as u8, (value >> 8) as u8, value as u8)
    }

    pub fn near(self, other: Rgb, tolerance: u8) -> bool {
        self.0.abs_diff(other.0) <= tolerance
            && self.1.abs_diff(other.1) <= tolerance
            && self.2.abs_diff(other.2) <= tolerance
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }
}

#[derive(Clone)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    rgba: Vec<u8>,
}

impl Image {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        assert_eq!(rgba.len(), (width * height * 4) as usize);
        Self { width, height, rgba }
    }

    pub fn pixel(&self, x: u32, y: u32) -> Rgb {
        let at = ((y * self.width + x) * 4) as usize;
        Rgb(self.rgba[at], self.rgba[at + 1], self.rgba[at + 2])
    }

    pub fn load(path: &Path) -> Option<Self> {
        let decoder = png::Decoder::new(std::io::BufReader::new(File::open(path).ok()?));
        let mut reader = decoder.read_info().unwrap();
        let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buffer).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba, "baselines are RGBA");
        buffer.truncate(info.buffer_size());
        Some(Self::new(info.width, info.height, buffer))
    }

    pub fn save(&self, path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut encoder = png::Encoder::new(BufWriter::new(File::create(path).unwrap()), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&self.rgba).unwrap();
    }

    /// Connected areas of `colour` (within `tolerance` per channel) covering at least `min_area`
    /// pixels, as bounding boxes in reading order.
    pub fn blobs(&self, colour: Rgb, tolerance: u8, min_area: u32) -> Vec<Rect> {
        let (width, height) = (self.width as usize, self.height as usize);
        let mut seen = vec![false; width * height];
        let mut found = Vec::new();
        for start in 0..width * height {
            if seen[start] || !self.pixel((start % width) as u32, (start / width) as u32).near(colour, tolerance) {
                continue;
            }
            let (mut left, mut top, mut right, mut bottom, mut area) = (width, height, 0, 0, 0u32);
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(at) = stack.pop() {
                let (x, y) = (at % width, at / width);
                (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
                area += 1;
                let neighbours = [
                    (x > 0).then(|| at - 1),
                    (x + 1 < width).then(|| at + 1),
                    (y > 0).then(|| at - width),
                    (y + 1 < height).then(|| at + width),
                ];
                for next in neighbours.into_iter().flatten() {
                    if !seen[next] && self.pixel((next % width) as u32, (next / width) as u32).near(colour, tolerance) {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
            if area >= min_area {
                found.push(Rect {
                    x: left as u32,
                    y: top as u32,
                    width: (right - left + 1) as u32,
                    height: (bottom - top + 1) as u32,
                });
            }
        }
        found.sort_by_key(|rect| (rect.y / 8, rect.x));
        found
    }

    /// How many pixels differ by more than `tolerance` in some channel, and an image marking them.
    pub fn difference(&self, other: &Image, tolerance: u8) -> (usize, Image) {
        self.difference_within(other, tolerance, 0)
    }

    /// [`Image::difference`], where a pixel matching `other` up to `shift` rows above or below
    /// counts as the same: at a fractional display scale a line of text can snap one physical
    /// pixel either way between runs.
    pub fn difference_within(&self, other: &Image, tolerance: u8, shift: u32) -> (usize, Image) {
        assert_eq!((self.width, self.height), (other.width, other.height), "the screens differ in size");
        let mut changed = 0;
        let mut marked = Vec::with_capacity(self.rgba.len());
        for (index, mine) in self.rgba.as_chunks::<4>().0.iter().enumerate() {
            let (x, y) = (index as u32 % self.width, index as u32 / self.width);
            let rows = y.saturating_sub(shift)..=(y + shift).min(self.height - 1);
            let differs = rows.into_iter().all(|row| {
                let at = ((row * self.width + x) * 4) as usize;
                mine.iter().zip(&other.rgba[at..at + 3]).any(|(a, b)| a.abs_diff(*b) > tolerance)
            });
            if differs {
                changed += 1;
                marked.extend_from_slice(&[255, 0, 255, 255]);
            } else {
                // the unchanged screen, faded, so the marks stand out
                marked.extend(mine.iter().take(3).map(|channel| 128 + channel / 2).chain([255]));
            }
        }
        (changed, Image::new(self.width, self.height, marked))
    }

    /// The smallest rectangle holding every pixel that differs from `other` at all, or `None` when
    /// the two are the same.
    pub fn changed_area(&self, other: &Image) -> Option<Rect> {
        assert_eq!((self.width, self.height), (other.width, other.height), "the screens differ in size");
        let pixels = self.rgba.as_chunks::<4>().0.iter().zip(other.rgba.as_chunks::<4>().0);
        let changed = pixels.enumerate().filter(|(_, (mine, theirs))| mine[..3] != theirs[..3]);
        let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
        for (index, _) in changed {
            let (x, y) = (index as u32 % self.width, index as u32 / self.width);
            (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
        }
        (left <= right).then(|| Rect { x: left, y: top, width: right - left + 1, height: bottom - top + 1 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(width: u32, height: u32) -> Image {
        Image::new(width, height, [128, 128, 128, 255].repeat((width * height) as usize))
    }

    fn with_dot(mut image: Image, x: u32, y: u32) -> Image {
        let at = ((y * image.width + x) * 4) as usize;
        image.rgba[at..at + 3].copy_from_slice(&[0, 0, 0]);
        image
    }

    #[test]
    fn the_same_screen_has_no_changed_area() {
        assert_eq!(grey(4, 3).changed_area(&grey(4, 3)), None);
    }

    #[test]
    fn the_changed_area_spans_every_changed_pixel() {
        let changed = with_dot(with_dot(grey(10, 8), 2, 6), 7, 1);
        assert_eq!(changed.changed_area(&grey(10, 8)), Some(Rect { x: 2, y: 1, width: 6, height: 6 }));
    }

    #[test]
    fn a_pixel_moved_one_row_counts_unless_a_shift_is_allowed() {
        let (before, after) = (with_dot(grey(5, 6), 2, 2), with_dot(grey(5, 6), 2, 3));
        assert_eq!(after.difference(&before, 0).0, 2);
        assert_eq!(after.difference_within(&before, 0, 1).0, 0);
    }

    #[test]
    fn a_pixel_moved_two_rows_counts_even_with_a_one_row_shift() {
        let (before, after) = (with_dot(grey(5, 6), 2, 1), with_dot(grey(5, 6), 2, 3));
        // where the dot is now nothing near matches; where it was, the grey above still does
        assert_eq!(after.difference_within(&before, 0, 1).0, 1);
    }

    #[test]
    fn one_changed_pixel_is_a_one_pixel_area() {
        assert_eq!(
            with_dot(grey(5, 5), 4, 4).changed_area(&grey(5, 5)),
            Some(Rect { x: 4, y: 4, width: 1, height: 1 })
        );
    }
}
