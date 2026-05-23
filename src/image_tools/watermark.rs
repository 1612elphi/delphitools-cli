use crate::error::Error;
use crate::image_tools::{derive_output, open_image, resolve_output, run_batch, save_rgba, BatchOk};
use image::imageops::FilterType;
use image::{Rgba, RgbaImage};
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_arguments)]
pub fn run(
    images: &[PathBuf],
    mark: &Path,
    position: &str,
    opacity: f32,
    scale: f32,
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if !(0.0..=1.0).contains(&opacity) {
        return Err(Error::Usage(format!(
            "opacity must be between 0 and 1, got {opacity}"
        )));
    }
    if !(0.001..=1.0).contains(&scale) {
        return Err(Error::Usage(format!(
            "scale must be between 0.001 and 1.0, got {scale}"
        )));
    }

    let pos = parse_position(position)?;

    if !mark.exists() {
        return Err(Error::Input(format!("watermark not found: {}", mark.display())));
    }
    let mark_img = open_image(mark)?.to_rgba8();

    let n = images.len();
    run_batch(images, json, quiet, |input| {
        let img = open_image(input)?;
        let mut canvas = img.to_rgba8();
        let (cw, ch) = canvas.dimensions();
        let long = cw.max(ch);

        // Watermark sized so its longest edge = `scale * long`.
        let wm_long = ((long as f32) * scale).round().max(1.0) as u32;
        let (mw, mh) = (mark_img.width(), mark_img.height());
        let (rw, rh) = if mw >= mh {
            let new_w = wm_long;
            let new_h = ((mh as f32 / mw as f32) * new_w as f32).round().max(1.0) as u32;
            (new_w, new_h)
        } else {
            let new_h = wm_long;
            let new_w = ((mw as f32 / mh as f32) * new_h as f32).round().max(1.0) as u32;
            (new_w, new_h)
        };
        let resized = image::imageops::resize(&mark_img, rw, rh, FilterType::Lanczos3);

        // Padding: 3% of longest edge.
        let pad = ((long as f32) * 0.03).round() as u32;
        let (x, y) = position_offset(pos, cw, ch, rw, rh, pad);

        composite(&mut canvas, &resized, x as i32, y as i32, opacity);

        // Preserve original extension where possible.
        let derived = derive_output(input, "watermarked", None);
        // Convert RGBA to RGB if writing to a format that doesn't support alpha (jpg, bmp).
        let out_path = resolve_output(output, n, &derived)?;
        save_rgba(&canvas, &out_path)?;
        Ok(BatchOk::one(out_path))
    })
}

#[derive(Clone, Copy)]
enum Position {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

fn parse_position(s: &str) -> Result<Position, Error> {
    match s.to_ascii_lowercase().as_str() {
        "top-left" | "tl" | "topleft" => Ok(Position::TopLeft),
        "top" | "tc" | "t" => Ok(Position::Top),
        "top-right" | "tr" | "topright" => Ok(Position::TopRight),
        "left" | "ml" | "l" => Ok(Position::Left),
        "center" | "centre" | "mc" | "c" => Ok(Position::Center),
        "right" | "mr" | "r" => Ok(Position::Right),
        "bottom-left" | "bl" | "bottomleft" => Ok(Position::BottomLeft),
        "bottom" | "bc" | "b" => Ok(Position::Bottom),
        "bottom-right" | "br" | "bottomright" => Ok(Position::BottomRight),
        other => Err(Error::Usage(format!("invalid position: {other}"))),
    }
}

fn position_offset(p: Position, cw: u32, ch: u32, ww: u32, wh: u32, pad: u32) -> (u32, u32) {
    let max_x = cw.saturating_sub(ww);
    let max_y = ch.saturating_sub(wh);
    let pad_x = pad.min(max_x / 2);
    let pad_y = pad.min(max_y / 2);
    match p {
        Position::TopLeft => (pad_x, pad_y),
        Position::Top => (max_x / 2, pad_y),
        Position::TopRight => (max_x.saturating_sub(pad_x), pad_y),
        Position::Left => (pad_x, max_y / 2),
        Position::Center => (max_x / 2, max_y / 2),
        Position::Right => (max_x.saturating_sub(pad_x), max_y / 2),
        Position::BottomLeft => (pad_x, max_y.saturating_sub(pad_y)),
        Position::Bottom => (max_x / 2, max_y.saturating_sub(pad_y)),
        Position::BottomRight => (max_x.saturating_sub(pad_x), max_y.saturating_sub(pad_y)),
    }
}

fn composite(base: &mut RgbaImage, top: &RgbaImage, x: i32, y: i32, opacity: f32) {
    let (bw, bh) = base.dimensions();
    let (tw, th) = top.dimensions();
    for ty in 0..th {
        for tx in 0..tw {
            let bx = x + tx as i32;
            let by = y + ty as i32;
            if bx < 0 || by < 0 || bx >= bw as i32 || by >= bh as i32 {
                continue;
            }
            let src = *top.get_pixel(tx, ty);
            let a = src.0[3] as f32 / 255.0 * opacity;
            if a <= 0.0 {
                continue;
            }
            let dst = *base.get_pixel(bx as u32, by as u32);
            let inv = 1.0 - a;
            let r = (src.0[0] as f32 * a + dst.0[0] as f32 * inv).round() as u8;
            let g = (src.0[1] as f32 * a + dst.0[1] as f32 * inv).round() as u8;
            let b = (src.0[2] as f32 * a + dst.0[2] as f32 * inv).round() as u8;
            // Output alpha: keep the max of source contribution and existing.
            let dst_a = dst.0[3] as f32 / 255.0;
            let out_a = (a + dst_a * inv).clamp(0.0, 1.0);
            base.put_pixel(
                bx as u32,
                by as u32,
                Rgba([r, g, b, (out_a * 255.0).round() as u8]),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use std::fs;

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("delphi-wm-test-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_image(path: &Path, w: u32, h: u32, rgba: [u8; 4]) {
        let mut img = RgbaImage::new(w, h);
        for p in img.pixels_mut() {
            p.0 = rgba;
        }
        img.save(path).unwrap();
    }

    #[test]
    fn watermark_runs() {
        let dir = tmpdir("a");
        let base = dir.join("base.png");
        let mark = dir.join("mark.png");
        make_image(&base, 400, 200, [10, 20, 30, 255]);
        make_image(&mark, 50, 50, [255, 0, 0, 255]);
        run(
            &[base.clone()],
            &mark,
            "bottom-right",
            0.5,
            0.2,
            false,
            true,
            None,
        )
        .unwrap();
        let out = dir.join("base-watermarked.png");
        let img = image::open(&out).unwrap().to_rgba8();
        // Watermark sits offset from the bottom-right by ~3% padding.
        // Sample somewhere inside the watermark rectangle.
        let p = img.get_pixel(img.width() - 50, img.height() - 30).0;
        assert!(p[0] > 50, "expected red contribution, got {:?}", p);
    }

    #[test]
    fn rejects_bad_opacity() {
        let r = run(&[], Path::new("x"), "br", 2.0, 0.2, false, true, None);
        assert!(r.is_err());
    }

    #[test]
    fn parse_positions_accept_short_forms() {
        assert!(parse_position("br").is_ok());
        assert!(parse_position("BOTTOM-RIGHT").is_ok());
        assert!(parse_position("centre").is_ok());
    }
}
