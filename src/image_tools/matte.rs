use crate::colour::Colour;
use crate::error::Error;
use crate::image_tools::{derive_output, open_image, parse_ratio, resolve_output, run_batch, BatchOk};
use image::imageops::FilterType;
use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::filter::gaussian_blur_f32;
use std::path::{Path, PathBuf};

pub fn run(
    images: &[PathBuf],
    ratio: &str,
    mode: &str,
    colour: &str,
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    let (rw, rh) = parse_ratio(ratio)?;
    let mode = parse_mode(mode)?;
    let fill_col = if matches!(mode, MatteMode::Solid) {
        Some(Colour::parse(colour)?)
    } else {
        None
    };
    let n = images.len();

    run_batch(images, json, quiet, |input| {
        let img = open_image(input)?;
        let result = build_matte(&img, rw, rh, mode, fill_col)?;

        let derived = derive_output(input, "matted", Some("png"));
        let out_path = resolve_output(output, n, &derived)?;
        result
            .save(&out_path)
            .map_err(|e| Error::Processing(format!("could not save {}: {e}", out_path.display())))?;
        Ok(BatchOk::one(out_path).with_extras(serde_json::json!({
            "width": result.width(),
            "height": result.height(),
        })))
    })
}

#[derive(Clone, Copy)]
enum MatteMode {
    Solid,
    Blur,
    Gradient,
}

fn parse_mode(s: &str) -> Result<MatteMode, Error> {
    match s.to_ascii_lowercase().as_str() {
        "solid" | "colour" | "color" => Ok(MatteMode::Solid),
        "blur" | "blurred" => Ok(MatteMode::Blur),
        "gradient" | "grad" => Ok(MatteMode::Gradient),
        other => Err(Error::Usage(format!("invalid matte mode: {other}"))),
    }
}

/// Build a matte. Output dimensions: width × height where the longer side targets
/// at least the input's longer side and ratio is satisfied. Specifically we pick
/// the larger of (input.longer_side) and 1080 as the output's longer side, then
/// derive the shorter from the ratio.
fn build_matte(
    img: &DynamicImage,
    rw: f64,
    rh: f64,
    mode: MatteMode,
    fill: Option<Colour>,
) -> Result<RgbaImage, Error> {
    let iw = img.width();
    let ih = img.height();

    // Determine output dimensions: use the longer of (input edge, 1080) as the long side.
    let long = iw.max(ih).max(1080) as f64;
    let (ow, oh) = if rw >= rh {
        (long, (long * rh / rw).round())
    } else {
        ((long * rw / rh).round(), long)
    };
    let ow = ow.round().max(1.0) as u32;
    let oh = oh.round().max(1.0) as u32;

    let mut canvas: RgbaImage = match mode {
        MatteMode::Solid => {
            let c = fill.unwrap_or_else(|| Colour::from_u8(255, 255, 255));
            let (r, g, b) = c.to_u8();
            let a = (c.a.clamp(0.0, 1.0) * 255.0).round() as u8;
            let mut canvas = RgbaImage::new(ow, oh);
            for p in canvas.pixels_mut() {
                p.0 = [r, g, b, a];
            }
            canvas
        }
        MatteMode::Blur => {
            // Cover-fit the input image into ow×oh, slightly oversized (1.2×), then blur heavily.
            let scale = (ow as f32 / iw as f32).max(oh as f32 / ih as f32) * 1.2;
            let sw = ((iw as f32 * scale).round() as u32).max(1);
            let sh = ((ih as f32 * scale).round() as u32).max(1);
            let scaled = img.resize_exact(sw, sh, FilterType::Triangle).to_rgba8();
            let sigma = ((ow.max(oh) as f32) * 0.03).max(8.0);
            let blurred = gaussian_blur_f32(&scaled, sigma);

            // Crop centered to ow×oh
            let off_x = (sw.saturating_sub(ow)) / 2;
            let off_y = (sh.saturating_sub(oh)) / 2;
            let mut canvas = RgbaImage::new(ow, oh);
            for y in 0..oh {
                for x in 0..ow {
                    let sx = (off_x + x).min(sw - 1);
                    let sy = (off_y + y).min(sh - 1);
                    canvas.put_pixel(x, y, *blurred.get_pixel(sx, sy));
                }
            }
            canvas
        }
        MatteMode::Gradient => {
            // Sample dominant colour by averaging a downsample to 1×1.
            let dominant = img.resize_exact(1, 1, FilterType::Triangle).to_rgba8();
            let pix = dominant.get_pixel(0, 0).0;
            let base = (pix[0], pix[1], pix[2]);
            let dark = (
                base.0.saturating_sub(30),
                base.1.saturating_sub(30),
                base.2.saturating_sub(30),
            );
            // Diagonal linear gradient from base → dark
            let mut c = RgbaImage::new(ow, oh);
            let max = (ow as f32 + oh as f32).max(1.0);
            for y in 0..oh {
                for x in 0..ow {
                    let t = ((x as f32 + y as f32) / max).clamp(0.0, 1.0);
                    let r = lerp(base.0, dark.0, t);
                    let g = lerp(base.1, dark.1, t);
                    let b = lerp(base.2, dark.2, t);
                    c.put_pixel(x, y, Rgba([r, g, b, 255]));
                }
            }
            c
        }
    };

    // Inset image with padding ≈ 4% of long edge.
    let padding = (ow.max(oh) as f32 * 0.037).round() as u32;
    let avail_w = ow.saturating_sub(padding * 2).max(1);
    let avail_h = oh.saturating_sub(padding * 2).max(1);
    let scale = (avail_w as f32 / iw as f32).min(avail_h as f32 / ih as f32);
    let sw = ((iw as f32 * scale).round() as u32).max(1);
    let sh = ((ih as f32 * scale).round() as u32).max(1);
    let resized = img.resize_exact(sw, sh, FilterType::Lanczos3);
    let resized_rgba = resized.to_rgba8();

    let dx = (ow - sw) / 2;
    let dy = (oh - sh) / 2;
    // Composite respecting alpha (image on top).
    for y in 0..sh {
        for x in 0..sw {
            let src = *resized_rgba.get_pixel(x, y);
            let a = src.0[3];
            if a == 0 {
                continue;
            }
            if a == 255 {
                canvas.put_pixel(dx + x, dy + y, src);
            } else {
                let dst = *canvas.get_pixel(dx + x, dy + y);
                let af = a as f32 / 255.0;
                let bf = 1.0 - af;
                let r = (src.0[0] as f32 * af + dst.0[0] as f32 * bf).round() as u8;
                let g = (src.0[1] as f32 * af + dst.0[1] as f32 * bf).round() as u8;
                let b = (src.0[2] as f32 * af + dst.0[2] as f32 * bf).round() as u8;
                canvas.put_pixel(dx + x, dy + y, Rgba([r, g, b, 255]));
            }
        }
    }

    Ok(canvas)
}

#[inline]
fn lerp(a: u8, b: u8, t: f32) -> u8 {
    let r = a as f32 + (b as f32 - a as f32) * t;
    r.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    fn make_dynamic(w: u32, h: u32, rgb: [u8; 3]) -> DynamicImage {
        let mut img = RgbaImage::new(w, h);
        for p in img.pixels_mut() {
            p.0 = [rgb[0], rgb[1], rgb[2], 255];
        }
        DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn solid_matte_square() {
        let img = make_dynamic(400, 200, [100, 100, 100]);
        let out = build_matte(&img, 1.0, 1.0, MatteMode::Solid, Some(Colour::from_u8(0, 255, 0))).unwrap();
        assert_eq!(out.width(), out.height());
        // Corner pixel should be green-ish (the fill colour).
        let p = out.get_pixel(0, 0).0;
        assert_eq!(p[1], 255);
        assert_eq!(p[0], 0);
    }

    #[test]
    fn matte_portrait_ratio() {
        let img = make_dynamic(300, 300, [100, 100, 100]);
        let out = build_matte(&img, 4.0, 5.0, MatteMode::Solid, Some(Colour::from_u8(0, 0, 0))).unwrap();
        // 4:5 portrait → height > width
        assert!(out.height() > out.width());
        let aspect = out.width() as f64 / out.height() as f64;
        assert!((aspect - 4.0 / 5.0).abs() < 0.01);
    }

    #[test]
    fn parse_mode_alternatives() {
        assert!(matches!(parse_mode("solid").unwrap(), MatteMode::Solid));
        assert!(matches!(parse_mode("colour").unwrap(), MatteMode::Solid));
        assert!(matches!(parse_mode("blur").unwrap(), MatteMode::Blur));
        assert!(matches!(parse_mode("gradient").unwrap(), MatteMode::Gradient));
        assert!(parse_mode("nope").is_err());
    }
}
