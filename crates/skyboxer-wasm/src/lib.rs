use wasm_bindgen::prelude::*;
use image::imageops;

/// Convert an equirectangular panorama image to a skybox (6 cube faces stacked vertically).
///
/// `enhance` enables 2x Lanczos3 upscale + unsharpmask for better quality.
/// `format` determines the output face order: `"bevy"` or `"unity"`.
#[wasm_bindgen]
pub fn skybox(input: &[u8], rotation: f64, max_width: u32, enhance: bool, format: &str) -> Result<Vec<u8>, JsError> {
    let mut img = image::load_from_memory(input)
        .map_err(|e| JsError::new(&format!("Failed to decode image: {}", e)))?;

    if enhance {
        // Always upscale — scale factor based on image width, targeting 16384px (4096 face width)
        let w = img.width();
        let h = img.height();
        let target = 16384u32;
        let scale = (target / w).max(1).min(8); // min 2x, max 8x
        let new_w = w * scale;
        let new_h = h * scale;

        img = match &img {
            image::DynamicImage::ImageRgba16(r) => {
                image::DynamicImage::ImageRgba16(imageops::resize(r, new_w, new_h, imageops::FilterType::Lanczos3))
            }
            image::DynamicImage::ImageRgb8(r) => {
                image::DynamicImage::ImageRgb8(imageops::resize(r, new_w, new_h, imageops::FilterType::Lanczos3))
            }
            _ => {
                let resized = imageops::resize(&img, new_w, new_h, imageops::FilterType::Lanczos3);
                image::DynamicImage::ImageRgba8(resized)
            }
        };

        // Sharpen after upscale to counteract interpolation softness
        img = match &img {
            image::DynamicImage::ImageRgba16(r) => {
                image::DynamicImage::ImageRgba16(imageops::unsharpen(r, 1.5, 1))
            }
            image::DynamicImage::ImageRgb8(r) => {
                image::DynamicImage::ImageRgb8(imageops::unsharpen(r, 1.5, 1))
            }            
            _ => {
                let sharpened = imageops::unsharpen(&img, 1.5, 1);
                image::DynamicImage::ImageRgba8(sharpened)
            }
        };
    }

    use skyboxer::SkyboxType as Ty;
    let ty = match format {
        "unity" => Ty::Unity,
        _ => Ty::Bevy
    };
    let result = ty.convert_panorama(&img, rotation, max_width);

    // Encode to PNG (preserve bit depth)
    let mut png_bytes: Vec<u8> = Vec::new();
    let depth = match &result {
        image::DynamicImage::ImageLuma8(_) | image::DynamicImage::ImageLumaA8(_)
        | image::DynamicImage::ImageRgb8(_) | image::DynamicImage::ImageRgba8(_) => 8u16,
        _ => 16,
    };

    if depth == 16 {
        let rgba16 = result.to_rgba16();
        let mut encoder = png::Encoder::new(&mut png_bytes, rgba16.width(), rgba16.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder
            .write_header()
            .map_err(|e| JsError::new(&format!("PNG header error: {}", e)))?;

        // PNG 16-bit expects big-endian; as_raw() gives native (LE) byte order
        let raw = rgba16.as_raw();
        let mut big_endian = vec![0u8; raw.len() * 2];
        for (i, val) in raw.iter().enumerate() {
            let base = i * 2;
            big_endian[base] = (val >> 8) as u8;
            big_endian[base + 1] = (val & 0xff) as u8;
        }
        writer
            .write_image_data(&big_endian)
            .map_err(|e| JsError::new(&format!("PNG data error: {}", e)))?;
    } else {
        result
            .write_to(
                &mut std::io::Cursor::new(&mut png_bytes),
                image::ImageFormat::Png,
            )
            .map_err(|e| JsError::new(&format!("PNG encode error: {}", e)))?;
    }

    Ok(png_bytes)
}

/// Apply all filters together.
///
/// Parameters:
/// - input: raw image bytes
/// - invert: invert colors
/// - contrast: contrast factor (-100 to 100)
/// - brighten: brightness adjustment (-255 to 255)
/// - hue_rotate: hue rotation in degrees (0-360)
#[wasm_bindgen]
pub fn apply_filters(
    input: &[u8],
    invert: bool,
    contrast: f32,
    brighten: i32,
    hue_rotate: i32,
) -> Result<Vec<u8>, JsError> {
    let mut img = image::load_from_memory(input)
        .map_err(|e| JsError::new(&format!("Failed to decode image: {}", e)))?;

    // 1. Invert
    if invert {
        image::imageops::colorops::invert(&mut img);
    }

    // 3. Contrast (RGB only, preserve alpha)
    if contrast != 0.0 {
        let factor = (1.0 + contrast / 100.0).powi(2);
        match &mut img {
            image::DynamicImage::ImageRgba8(ref mut data) => {
                for pixel in data.pixels_mut() {
                    for channel in 0..3 {
                        let v = pixel[channel] as f32 / 255.0 - 0.5;
                        pixel[channel] = ((v * factor + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
                    }
                }
            }
            image::DynamicImage::ImageRgba16(ref mut data) => {
                for pixel in data.pixels_mut() {
                    for channel in 0..3 {
                        let v = pixel[channel] as f32 / 65535.0 - 0.5;
                        pixel[channel] = ((v * factor + 0.5) * 65535.0).clamp(0.0, 65535.0) as u16;
                    }
                }
            }
            _ => {
                image::imageops::colorops::contrast_in_place(&mut img, contrast);
            }
        }
    }

    // 4. Brightness (RGB only, preserve alpha)
    if brighten != 0 {
        match &mut img {
            image::DynamicImage::ImageRgba8(ref mut data) => {
                for pixel in data.pixels_mut() {
                    for channel in 0..3 {
                        pixel[channel] = (pixel[channel] as i32 + brighten).clamp(0, 255) as u8;
                    }
                }
            }
            image::DynamicImage::ImageRgba16(ref mut data) => {
                let b16 = (brighten as f32 / 255.0 * 65535.0) as i32;
                for pixel in data.pixels_mut() {
                    for channel in 0..3 {
                        pixel[channel] = (pixel[channel] as i32 + b16).clamp(0, 65535) as u16;
                    }
                }
            }
            _ => {
                image::imageops::colorops::brighten_in_place(&mut img, brighten);
            }
        }
    }

    // 5. Hue rotate (RGB only, preserve alpha)
    if hue_rotate != 0 {
        match &mut img {
            image::DynamicImage::ImageRgba8(ref mut data) => {
                for pixel in data.pixels_mut() {
                    let r = pixel[0] as f32 / 255.0;
                    let g = pixel[1] as f32 / 255.0;
                    let b = pixel[2] as f32 / 255.0;
                    let (h, s, l) = rgb_to_hsl(r, g, b);
                    let new_h = (h + hue_rotate as f32 / 360.0) % 1.0;
                    let (nr, ng, nb) = hsl_to_rgb(new_h, s, l);
                    pixel[0] = (nr * 255.0).round() as u8;
                    pixel[1] = (ng * 255.0).round() as u8;
                    pixel[2] = (nb * 255.0).round() as u8;
                }
            }
            image::DynamicImage::ImageRgba16(ref mut data) => {
                for pixel in data.pixels_mut() {
                    let r = pixel[0] as f32 / 65535.0;
                    let g = pixel[1] as f32 / 65535.0;
                    let b = pixel[2] as f32 / 65535.0;
                    let (h, s, l) = rgb_to_hsl(r, g, b);
                    let new_h = (h + hue_rotate as f32 / 360.0) % 1.0;
                    let (nr, ng, nb) = hsl_to_rgb(new_h, s, l);
                    pixel[0] = (nr * 65535.0).clamp(0.0, 65535.0) as u16;
                    pixel[1] = (ng * 65535.0).clamp(0.0, 65535.0) as u16;
                    pixel[2] = (nb * 65535.0).clamp(0.0, 65535.0) as u16;
                }
            }
            _ => {
                image::imageops::colorops::huerotate_in_place(&mut img, hue_rotate);
            }
        }
    }

    // Encode to PNG
    let mut png_bytes: Vec<u8> = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )
    .map_err(|e| JsError::new(&format!("PNG encode error: {}", e)))?;

    Ok(png_bytes)
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r { (g - b) / d + if g < b { 6.0 } else { 0.0 } }
        else if max == g { (b - r) / d + 2.0 }
        else { (r - g) / d + 4.0 };
    (h / 6.0, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s == 0.0 {
        let v = l;
        return (v, v, v);
    }
    let hue2rgb = |p: f32, q: f32, t: f32| -> f32 {
        let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
        if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
        if t < 1.0 / 2.0 { return q; }
        if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
        p
    };
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    (hue2rgb(p, q, h + 1.0 / 3.0), hue2rgb(p, q, h), hue2rgb(p, q, h - 1.0 / 3.0))
}

