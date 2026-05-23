use crate::error::Error;
use crate::image_tools::{derive_output, open_image, resolve_output, run_batch, BatchOk};
use image::RgbaImage;
use std::path::{Path, PathBuf};

pub fn run(
    images: &[PathBuf],
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    let n = images.len();
    run_batch(images, json, quiet, |input| {
        let img = open_image(input)?.to_rgba8();

        let (ow, oh) = (img.width(), img.height());
        let bounds = find_bounds(&img).ok_or_else(|| {
            Error::Processing(format!(
                "image is fully transparent — nothing to clip: {}",
                input.display()
            ))
        })?;
        let (top, right, bottom, left) = bounds;
        let cw = ow - left - right;
        let ch = oh - top - bottom;

        let clipped = if cw == ow && ch == oh {
            img
        } else {
            let mut out = RgbaImage::new(cw, ch);
            for y in 0..ch {
                for x in 0..cw {
                    let p = *img.get_pixel(x + left, y + top);
                    out.put_pixel(x, y, p);
                }
            }
            out
        };

        let derived = derive_output(input, "clipped", Some("png"));
        let out_path = resolve_output(output, n, &derived)?;
        clipped
            .save(&out_path)
            .map_err(|e| Error::Processing(format!("could not save {}: {e}", out_path.display())))?;
        Ok(BatchOk::one(out_path).with_extras(serde_json::json!({
            "original_width": ow,
            "original_height": oh,
            "clipped_width": cw,
            "clipped_height": ch,
            "trimmed": {
                "top": top,
                "right": right,
                "bottom": bottom,
                "left": left,
            },
        })))
    })
}

/// Return (top, right, bottom, left) of fully-transparent rows/cols that can be stripped.
/// Returns None if the entire image is transparent.
fn find_bounds(img: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (w, h) = (img.width(), img.height());
    let mut top = h;
    let mut bottom = 0;
    let mut left = w;
    let mut right = 0;
    let mut any = false;
    for y in 0..h {
        for x in 0..w {
            let a = img.get_pixel(x, y).0[3];
            if a > 0 {
                any = true;
                if y < top {
                    top = y;
                }
                if y > bottom {
                    bottom = y;
                }
                if x < left {
                    left = x;
                }
                if x > right {
                    right = x;
                }
            }
        }
    }
    if !any {
        return None;
    }
    Some((top, w - 1 - right, h - 1 - bottom, left))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use std::fs;

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("delphi-clip-test-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn clip_transparent_border() {
        // 20x20, with opaque 10x10 in the middle.
        let mut img = RgbaImage::new(20, 20);
        for y in 5..15 {
            for x in 5..15 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        let dir = tmpdir("a");
        let input = dir.join("in.png");
        img.save(&input).unwrap();
        run(&[input.clone()], false, true, None).unwrap();
        let out = dir.join("in-clipped.png");
        let clipped = image::open(&out).unwrap();
        assert_eq!(clipped.width(), 10);
        assert_eq!(clipped.height(), 10);
    }

    #[test]
    fn fully_opaque_passes_through() {
        let dir = tmpdir("b");
        let input = dir.join("in.png");
        let mut img = RgbaImage::new(8, 8);
        for p in img.pixels_mut() {
            p.0 = [10, 10, 10, 255];
        }
        img.save(&input).unwrap();
        run(&[input.clone()], false, true, None).unwrap();
        let out = dir.join("in-clipped.png");
        let clipped = image::open(&out).unwrap();
        assert_eq!(clipped.width(), 8);
        assert_eq!(clipped.height(), 8);
    }

    #[test]
    fn fully_transparent_errors() {
        let dir = tmpdir("c");
        let input = dir.join("in.png");
        let img = RgbaImage::new(4, 4); // all zeros = transparent
        img.save(&input).unwrap();
        let r = run(&[input], false, true, None);
        assert!(r.is_err());
    }
}
