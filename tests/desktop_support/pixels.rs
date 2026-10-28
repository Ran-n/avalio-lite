// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.790620
//+ Revised: 	2026/10/28 21:20:49.356837
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
        assert_eq!((self.width, self.height), (other.width, other.height), "the screens differ in size");
        let mut changed = 0;
        let mut marked = Vec::with_capacity(self.rgba.len());
        for (mine, theirs) in self.rgba.as_chunks::<4>().0.iter().zip(other.rgba.as_chunks::<4>().0) {
            let differs = mine.iter().zip(theirs).take(3).any(|(a, b)| a.abs_diff(*b) > tolerance);
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
}
