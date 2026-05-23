use crate::error::Error;
use crate::image_tools::{canonical_ext, flatten_alpha_to_rgb, open_image, resolve_output, run_batch, BatchOk};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageEncoder};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_arguments)]
pub fn run(
    images: &[PathBuf],
    to: &str,
    quality: u8,
    resize: Option<&str>,
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if !(1..=100).contains(&quality) {
        return Err(Error::Usage(format!(
            "quality must be 1-100, got {quality}"
        )));
    }
    let ext = canonical_ext(to);
    let target_fmt = parse_format(to)?;
    let n = images.len();

    run_batch(images, json, quiet, |input| {
        let img = open_image(input)?;

        let img = if let Some(spec) = resize {
            apply_resize(img, spec)?
        } else {
            img
        };

        // For convert we keep the original stem and change only the extension —
        // unlike other tools that append a suffix.
        let derived = swap_extension(input, ext);
        let out_path = resolve_output(output, n, &derived)?;
        // If output path equals input path, refuse to clobber.
        if out_path == *input {
            return Err(Error::Usage(format!(
                "input and output paths are identical: {}",
                input.display()
            )));
        }
        encode_to(&img, target_fmt, quality, &out_path)?;
        Ok(BatchOk::one(out_path).with_extras(serde_json::json!({
            "format": ext,
            "width": img.width(),
            "height": img.height(),
        })))
    })
}

#[derive(Clone, Copy)]
enum Format {
    Png,
    Jpeg,
    WebP,
    Gif,
    Tiff,
    Bmp,
    Ico,
}

fn parse_format(s: &str) -> Result<Format, Error> {
    match s.to_ascii_lowercase().as_str() {
        "png" => Ok(Format::Png),
        "jpg" | "jpeg" => Ok(Format::Jpeg),
        "webp" => Ok(Format::WebP),
        "gif" => Ok(Format::Gif),
        "tif" | "tiff" => Ok(Format::Tiff),
        "bmp" => Ok(Format::Bmp),
        "ico" => Ok(Format::Ico),
        other => Err(Error::Usage(format!("unsupported target format: {other}"))),
    }
}

fn swap_extension(input: &Path, new_ext: &str) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}.{new_ext}"))
}

fn apply_resize(img: DynamicImage, spec: &str) -> Result<DynamicImage, Error> {
    let s = spec.trim();
    if let Some(pct) = s.strip_suffix('%') {
        let p: f64 = pct
            .parse()
            .map_err(|_| Error::Usage(format!("invalid resize percentage: {spec}")))?;
        if p <= 0.0 || p > 1000.0 {
            return Err(Error::Usage(format!("resize percentage out of range: {spec}")));
        }
        let nw = ((img.width() as f64) * p / 100.0).round().max(1.0) as u32;
        let nh = ((img.height() as f64) * p / 100.0).round().max(1.0) as u32;
        return Ok(img.resize_exact(nw, nh, FilterType::Lanczos3));
    }
    if let Some((w_s, h_s)) = s.split_once('x').or_else(|| s.split_once('X')) {
        let aspect = img.width() as f64 / img.height() as f64;
        let (nw, nh) = match (w_s.trim(), h_s.trim()) {
            ("", "") => return Err(Error::Usage("resize: both dimensions empty".into())),
            (w, "") => {
                let w: u32 = w.parse().map_err(|_| Error::Usage(format!("bad width in resize: {spec}")))?;
                let h = ((w as f64) / aspect).round().max(1.0) as u32;
                (w, h)
            }
            ("", h) => {
                let h: u32 = h.parse().map_err(|_| Error::Usage(format!("bad height in resize: {spec}")))?;
                let w = ((h as f64) * aspect).round().max(1.0) as u32;
                (w, h)
            }
            (w, h) => {
                let w: u32 = w.parse().map_err(|_| Error::Usage(format!("bad width in resize: {spec}")))?;
                let h: u32 = h.parse().map_err(|_| Error::Usage(format!("bad height in resize: {spec}")))?;
                (w, h)
            }
        };
        return Ok(img.resize_exact(nw, nh, FilterType::Lanczos3));
    }
    Err(Error::Usage(format!("invalid resize spec: {spec}")))
}

fn encode_to(img: &DynamicImage, fmt: Format, quality: u8, path: &Path) -> Result<(), Error> {
    let f = std::fs::File::create(path)
        .map_err(|e| Error::Processing(format!("could not create {}: {e}", path.display())))?;
    let mut w = BufWriter::new(f);
    match fmt {
        Format::Jpeg => {
            // Flatten alpha onto white before encoding (JPEG has no alpha).
            let rgb = flatten_alpha_to_rgb(&img.to_rgba8());
            let mut enc = JpegEncoder::new_with_quality(&mut w, quality);
            enc.encode_image(&rgb)
                .map_err(|e| Error::Processing(format!("jpeg encode failed: {e}")))?;
        }
        Format::WebP => {
            // image 0.25's built-in WebP encoder is lossless-only; --quality is a no-op here.
            let rgba = img.to_rgba8();
            let enc = WebPEncoder::new_lossless(&mut w);
            enc.encode(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| Error::Processing(format!("webp encode failed: {e}")))?;
        }
        Format::Png => {
            let rgba = img.to_rgba8();
            let enc = PngEncoder::new(&mut w);
            enc.write_image(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| Error::Processing(format!("png encode failed: {e}")))?;
        }
        Format::Gif => {
            // Use DynamicImage::save_with_format via a temporary buffer to avoid double-open.
            // Easiest path: drop the BufWriter and let `image` save directly.
            drop(w);
            img.save_with_format(path, image::ImageFormat::Gif)
                .map_err(|e| Error::Processing(format!("gif encode failed: {e}")))?;
        }
        Format::Tiff => {
            drop(w);
            img.save_with_format(path, image::ImageFormat::Tiff)
                .map_err(|e| Error::Processing(format!("tiff encode failed: {e}")))?;
        }
        Format::Bmp => {
            // BMP via image crate; flatten alpha onto white.
            drop(w);
            let rgb = flatten_alpha_to_rgb(&img.to_rgba8());
            rgb.save_with_format(path, image::ImageFormat::Bmp)
                .map_err(|e| Error::Processing(format!("bmp encode failed: {e}")))?;
        }
        Format::Ico => {
            // For single-image ICO, use the `ico` crate directly so we control the format
            // robustly across image sizes.
            drop(w);
            let mut img2 = img.clone();
            // Crop to square + resize down to ≤256.
            let side = img2.width().min(img2.height());
            let x = (img2.width() - side) / 2;
            let y = (img2.height() - side) / 2;
            let square = img2.crop(x, y, side, side);
            let sz = side.min(256);
            let resized = square.resize_exact(sz, sz, FilterType::Lanczos3).to_rgba8();
            let icon = ico::IconImage::from_rgba_data(sz, sz, resized.as_raw().clone());
            let entry = ico::IconDirEntry::encode(&icon)
                .map_err(|e| Error::Processing(format!("ico encode failed: {e}")))?;
            let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
            dir.add_entry(entry);
            let f = std::fs::File::create(path).map_err(|e| {
                Error::Processing(format!("could not create {}: {e}", path.display()))
            })?;
            dir.write(f)
                .map_err(|e| Error::Processing(format!("ico write failed: {e}")))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use std::fs;

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("delphi-conv-test-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_image(path: &Path, w: u32, h: u32) {
        let mut img = RgbaImage::new(w, h);
        for p in img.pixels_mut() {
            p.0 = [50, 200, 100, 255];
        }
        img.save(path).unwrap();
    }

    #[test]
    fn resize_percentage() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 50));
        let out = apply_resize(img, "50%").unwrap();
        assert_eq!(out.width(), 50);
        assert_eq!(out.height(), 25);
    }

    #[test]
    fn resize_wxh() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 50));
        let out = apply_resize(img, "80x30").unwrap();
        assert_eq!(out.width(), 80);
        assert_eq!(out.height(), 30);
    }

    #[test]
    fn resize_w_only_keeps_aspect() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(200, 100));
        let out = apply_resize(img, "100x").unwrap();
        assert_eq!(out.width(), 100);
        assert_eq!(out.height(), 50);
    }

    #[test]
    fn resize_h_only_keeps_aspect() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(200, 100));
        let out = apply_resize(img, "x50").unwrap();
        assert_eq!(out.width(), 100);
        assert_eq!(out.height(), 50);
    }

    #[test]
    fn convert_png_to_jpg() {
        let dir = tmpdir("a");
        let input = dir.join("in.png");
        make_image(&input, 50, 50);
        run(&[input.clone()], "jpg", 85, None, false, true, Some(&dir)).unwrap();
        let out = dir.join("in.jpg");
        assert!(out.exists());
        let img = image::open(&out).unwrap();
        assert_eq!(img.width(), 50);
    }

    #[test]
    fn convert_with_resize() {
        let dir = tmpdir("b");
        let input = dir.join("in.png");
        make_image(&input, 100, 100);
        run(
            &[input.clone()],
            "png",
            85,
            Some("50%"),
            false,
            true,
            Some(&dir.join("scaled.png")),
        )
        .unwrap();
        let out = dir.join("scaled.png");
        let img = image::open(&out).unwrap();
        assert_eq!(img.width(), 50);
    }

    #[test]
    fn convert_to_webp() {
        let dir = tmpdir("c");
        let input = dir.join("in.png");
        make_image(&input, 32, 32);
        run(&[input.clone()], "webp", 80, None, false, true, Some(&dir)).unwrap();
        let out = dir.join("in.webp");
        assert!(out.exists());
        let img = image::open(&out).unwrap();
        assert_eq!(img.width(), 32);
    }

    #[test]
    fn refuses_clobber_when_no_format_change() {
        let dir = tmpdir("d");
        let input = dir.join("in.png");
        make_image(&input, 10, 10);
        // png -> png with no output specified would derive the same path; refuse.
        let r = run(&[input], "png", 85, None, false, true, None);
        assert!(r.is_err());
    }
}
