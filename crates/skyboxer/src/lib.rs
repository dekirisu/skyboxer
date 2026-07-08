//! Skybox conversion: turns an equirectangular panorama image into a skybox
//! (6 cube faces stacked vertically).
//!
//! This is the core skyboxer logic, extracted from the original skyboxer tool.

use deki_macros::*;
use image::{DynamicImage, EncodableLayout, ImageBuffer, Pixel, PixelWithColorType};
use std::f64::consts::PI;


// Face \\

#[derive(Default, Clone, Copy)]
pub struct CubeFace {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub const fn face(x: f64, y: f64, z: f64) -> CubeFace {
    CubeFace { x, y, z }
}

/// Define the orientations for each cube face
pub const ORIENTATIONS: [fn(f64, f64) -> CubeFace; 6] = [
    |x, y| face(-1., -x, -y), // +z
    |x, y| face(1., x, -y),   // -z
    |x, y| face(x, -1., -y),  // +x
    |x, y| face(-x, 1., -y),  // -x
    |x, y| face(-y, -x, 1.),  // +y
    |x, y| face(y, -x, -1.),  // -y
];

pub fn mod2pi(x: f64) -> f64 {
    let mut x = x % (2.0 * PI);
    if x < 0.0 {
        x += 2.0 * PI;
    }
    x
}

/// Render a single cube face from a panorama image
pub fn render_face<P: Pixel>(
    read_data: &ImageBuffer<P, Vec<P::Subpixel>>,
    face: usize,
    rotation: f64,
    _interpolation: &str,
    max_width: u32,
) -> ImageBuffer<P, Vec<P::Subpixel>> {
    let face_width = max_width.min(read_data.width() / 4);
    let face_width = face_width.min(4096);
    let face_height = face_width;

    let mut write_data = ImageBuffer::new(face_width, face_height);
    let orientation = ORIENTATIONS[face];

    for x in 0..face_width {
        for y in 0..face_height {
            let cube = orientation(
                2.0 * (x as f64 + 0.5) / face_width as f64 - 1.0,
                2.0 * (y as f64 + 0.5) / face_height as f64 - 1.0,
            );

            let r = (cube.x * cube.x + cube.y * cube.y + cube.z * cube.z).sqrt();
            let lon = mod2pi(cube.y.atan2(cube.x) + rotation);
            let lat = (cube.z / r).acos();

            let src_x = (read_data.width() as f64 * lon / (PI * 2.0) - 0.5) as u32;
            let src_y = (read_data.height() as f64 * lat / PI - 0.5) as u32;
            if src_x < read_data.width() && src_y < read_data.height() {
                let pixel = read_data.get_pixel(src_x, src_y);
                write_data.put_pixel(x, y, *pixel);
            }
        }
    }

    write_data
}


// Types \\

pub enum SkyboxType {
    /// 4×6 vertical strip — pos_x, neg_x, pos_y, neg_y, pos_z, neg_z
    Bevy,
    /// 4×3 cross layout
    Unity
}


/// Render all six cube faces.
///
/// `format` determines the output layout:
/// - `"bevy"`:  
/// - `"unity"`: :
///     [top]
/// [left][front][right][back]
///     [bottom]
#[imp(SkyboxType)]
pub fn render<P: PixelWithColorType>(
    &self,
    read_data: &ImageBuffer<P, Vec<P::Subpixel>>,
    rotation: f64,
    interpolation: &str,
    max_width: u32,
) -> ImageBuffer<P, Vec<P::Subpixel>>
where
    [P::Subpixel]: EncodableLayout,
{
    let renders = [0, 1, 2, 3, 4, 5].map(|x| render_face(read_data, x, rotation, interpolation, max_width));

    let [pos_z, neg_z, pos_x, neg_x, pos_y, neg_y] = renders;
    let h = pos_x.height();

    match self {
        SkyboxType::Unity => {
            // Unity cross layout: 4 columns × 3 rows
            // Row 0: [empty][top][empty][empty]
            // Row 1: [left][front][right][back]
            // Row 2: [empty][bottom][empty][empty]
            let mut out = ImageBuffer::<P, Vec<P::Subpixel>>::new(pos_x.width() * 4, h * 3);

            // Top (row 0, col 1)
            for (x, y, p) in pos_y.enumerate_pixels() {
                out.put_pixel(x + pos_x.width(), y, p.clone());
            }
            // Left (row 1, col 0)
            for (x, y, p) in neg_x.enumerate_pixels() {
                out.put_pixel(x, y + h, p.clone());
            }
            // Front / +z (row 1, col 1)
            for (x, y, p) in pos_z.enumerate_pixels() {
                out.put_pixel(x + pos_x.width(), y + h, p.clone());
            }
            // Right / +x (row 1, col 2)
            for (x, y, p) in pos_x.enumerate_pixels() {
                out.put_pixel(x + pos_x.width() * 2, y + h, p.clone());
            }
            // Back / -z (row 1, col 3)
            for (x, y, p) in neg_z.enumerate_pixels() {
                out.put_pixel(x + pos_x.width() * 3, y + h, p.clone());
            }
            // Bottom (row 2, col 1)
            for (x, y, p) in neg_y.enumerate_pixels() {
                out.put_pixel(x + pos_x.width(), y + h * 2, p.clone());
            }

            out
        }
        SkyboxType::Bevy => {
            // Bevy vertical strip: pos_x, neg_x, pos_y, neg_y, pos_z, neg_z
            let mut out = ImageBuffer::<P, Vec<P::Subpixel>>::new(pos_x.width(), h * 6);

            let mut idx: u32 = 0;
            for (x, y, p) in pos_x.enumerate_pixels() {
                out.put_pixel(x, y + h * idx, p.clone());
            }
            idx += 1;
            for (x, y, p) in neg_x.enumerate_pixels() {
                out.put_pixel(x, y + h * idx, p.clone());
            }
            idx += 1;
            for (x, y, p) in pos_y.enumerate_pixels() {
                out.put_pixel(x, y + h * idx, p.clone());
            }
            idx += 1;
            for (x, y, p) in neg_y.enumerate_pixels() {
                out.put_pixel(x, y + h * idx, p.clone());
            }
            idx += 1;
            for (x, y, p) in pos_z.enumerate_pixels() {
                out.put_pixel(x, y + h * idx, p.clone());
            }
            idx += 1;
            for (x, y, p) in neg_z.enumerate_pixels() {
                out.put_pixel(x, y + h * idx, p.clone());
            }

            out
        }
    }
}

/// Convert an equirectangular panorama image to a skybox (6 cube faces stacked vertically).
///
/// `format` determines the output face order: `"bevy"` or `"unity"`.
///
/// Returns the result as a `DynamicImage`.
#[imp(SkyboxType)]
pub fn convert_panorama(
    &self,
    input: &DynamicImage,
    rotation: f64,
    max_width: u32,
) -> DynamicImage {
    match input {
        DynamicImage::ImageLuma8(b)  => self.render(b, rotation, "linear", max_width).into(),
        DynamicImage::ImageLumaA8(b) => self.render(b, rotation, "linear", max_width).into(),
        DynamicImage::ImageRgb8(b)   => self.render(b, rotation, "linear", max_width).into(),
        DynamicImage::ImageRgba8(b)  => self.render(b, rotation, "linear", max_width).into(),
        DynamicImage::ImageLuma16(b) => self.render(b, rotation, "linear", max_width).into(),
        DynamicImage::ImageLumaA16(b)=> self.render(b, rotation, "linear", max_width).into(),
        DynamicImage::ImageRgb16(b)  => self.render(b, rotation, "linear", max_width).into(),
        DynamicImage::ImageRgba16(b) => self.render(b, rotation, "linear", max_width).into(),
        DynamicImage::ImageRgb32F(b) => self.render(b, rotation, "linear", max_width).into(),
        DynamicImage::ImageRgba32F(b)=> self.render(b, rotation, "linear", max_width).into(),
        _ => input.clone(),
    }
}
