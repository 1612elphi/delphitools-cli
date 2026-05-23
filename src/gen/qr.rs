use crate::colour::Colour;
use crate::error::Error;
use crate::image_tools::open_image;
use image::{imageops::FilterType, ImageBuffer, Rgba};
use qrcode::{EcLevel, QrCode, Version};
use serde_json::json;
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_arguments)]
pub fn run(
    data: &str,
    size: u32,
    fg: &str,
    bg: &str,
    logo: Option<&Path>,
    error_level: &str,
    json_out: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if data.is_empty() {
        return Err(Error::Usage("qr: empty data".into()));
    }
    if size < 32 {
        return Err(Error::Usage(format!(
            "qr: size {size} is too small (minimum 32)"
        )));
    }

    // Spec: when --logo is set, force error level H so the logo's punch-out
    // can be tolerated.
    let mut ec = parse_ec(error_level)?;
    if logo.is_some() {
        ec = EcLevel::H;
    }

    let code = QrCode::with_error_correction_level(data.as_bytes(), ec)
        .map_err(|e| Error::Input(format!("qr: {e}")))?;

    let modules: u32 = code
        .width()
        .try_into()
        .map_err(|_| Error::Processing("qr: width overflows u32".into()))?;

    // Normal QR codes use a 4-module quiet zone; Micro QR uses 2.
    let quiet_zone: u32 = if code.version().is_micro() { 2 } else { 4 };
    let total_modules = modules + 2 * quiet_zone;

    // Pixels per module; at least 1.
    let module_px: u32 = (size / total_modules).max(1);
    let canvas_px: u32 = module_px * total_modules;

    // Parse colours. "transparent" / "none" → fully transparent background.
    let fg_col = Colour::parse(fg)?;
    let (fr, fg_g, fb) = fg_col.to_u8();
    let fg_rgba = Rgba([fr, fg_g, fb, 255]);

    let bg_lower = bg.trim().to_ascii_lowercase();
    let bg_rgba = if bg_lower == "transparent" || bg_lower == "none" {
        Rgba([0, 0, 0, 0])
    } else {
        let bgc = Colour::parse(bg)?;
        let (br, bgg, bb) = bgc.to_u8();
        // Honour alpha so `--bg "#ffffff00"` matches `--bg transparent`.
        let ba = (bgc.a.clamp(0.0, 1.0) * 255.0).round() as u8;
        Rgba([br, bgg, bb, ba])
    };

    // Paint the canvas.
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(canvas_px, canvas_px, bg_rgba);
    let colors = code.to_colors();
    for my in 0..modules {
        for mx in 0..modules {
            let idx = (my * modules + mx) as usize;
            if colors[idx] == qrcode::Color::Dark {
                let px = (mx + quiet_zone) * module_px;
                let py = (my + quiet_zone) * module_px;
                for dy in 0..module_px {
                    for dx in 0..module_px {
                        img.put_pixel(px + dx, py + dy, fg_rgba);
                    }
                }
            }
        }
    }

    // Optional logo overlay (centred, ~22% of canvas).
    if let Some(logo_path) = logo {
        let logo_img = open_image(logo_path)?.to_rgba8();
        let target = (canvas_px * 22 / 100).max(1);
        // Fit the logo inside a `target` × `target` square, preserving aspect ratio.
        let (lw, lh) = logo_img.dimensions();
        let scale = target as f32 / lw.max(lh).max(1) as f32;
        let new_w = ((lw as f32) * scale).round().max(1.0).min(canvas_px as f32) as u32;
        let new_h = ((lh as f32) * scale).round().max(1.0).min(canvas_px as f32) as u32;
        let logo_scaled = image::imageops::resize(&logo_img, new_w, new_h, FilterType::Lanczos3);
        let ox = (canvas_px.saturating_sub(new_w)) / 2;
        let oy = (canvas_px.saturating_sub(new_h)) / 2;
        // Composite logo with alpha onto canvas.
        for y in 0..new_h {
            for x in 0..new_w {
                let lp = *logo_scaled.get_pixel(x, y);
                let alpha = lp[3];
                if alpha == 0 {
                    continue;
                }
                let dst_x = ox + x;
                let dst_y = oy + y;
                if dst_x >= canvas_px || dst_y >= canvas_px {
                    continue;
                }
                if alpha == 255 {
                    img.put_pixel(dst_x, dst_y, lp);
                } else {
                    let dst = *img.get_pixel(dst_x, dst_y);
                    let a = alpha as f32 / 255.0;
                    let blended = Rgba([
                        ((lp[0] as f32) * a + (dst[0] as f32) * (1.0 - a)).round() as u8,
                        ((lp[1] as f32) * a + (dst[1] as f32) * (1.0 - a)).round() as u8,
                        ((lp[2] as f32) * a + (dst[2] as f32) * (1.0 - a)).round() as u8,
                        // Destination keeps its alpha (so transparent bg stays transparent
                        // only where the logo doesn't cover; here we just take dst).
                        dst[3].max(alpha),
                    ]);
                    img.put_pixel(dst_x, dst_y, blended);
                }
            }
        }
    }

    let out_path: PathBuf = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("qr.png"));
    img.save(&out_path)
        .map_err(|e| Error::Processing(format!("qr: write {}: {e}", out_path.display())))?;

    let version_label = match code.version() {
        Version::Normal(v) => format!("{v}"),
        Version::Micro(v) => format!("M{v}"),
    };
    let ec_label = match ec {
        EcLevel::L => "L",
        EcLevel::M => "M",
        EcLevel::Q => "Q",
        EcLevel::H => "H",
    };

    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "path": out_path.display().to_string(),
                "modules": modules,
                "version": version_label,
                "error_level": ec_label,
            }))
            .unwrap()
        );
    } else if !quiet {
        println!("{}", out_path.display());
    }

    Ok(())
}

fn parse_ec(s: &str) -> Result<EcLevel, Error> {
    match s.trim().to_ascii_uppercase().as_str() {
        "L" => Ok(EcLevel::L),
        "M" => Ok(EcLevel::M),
        "Q" => Ok(EcLevel::Q),
        "H" => Ok(EcLevel::H),
        other => Err(Error::Usage(format!(
            "qr: invalid error-level {other}; expected L, M, Q, or H"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("delphi-qr-test-{}-{}.png", std::process::id(), name));
        p
    }

    #[test]
    fn parse_ec_levels() {
        assert!(matches!(parse_ec("L").unwrap(), EcLevel::L));
        assert!(matches!(parse_ec("m").unwrap(), EcLevel::M));
        assert!(matches!(parse_ec("Q").unwrap(), EcLevel::Q));
        assert!(matches!(parse_ec("h").unwrap(), EcLevel::H));
        assert!(parse_ec("X").is_err());
    }

    #[test]
    fn empty_data_is_rejected() {
        let p = tmp_path("empty");
        let err = run("", 256, "#000", "#fff", None, "M", false, true, Some(&p)).unwrap_err();
        assert!(matches!(err, Error::Usage(_)));
    }

    #[test]
    fn writes_a_valid_png() {
        let p = tmp_path("hello");
        run("hello world", 256, "#000", "#fff", None, "M", false, true, Some(&p)).unwrap();
        let bytes = fs::read(&p).expect("PNG was not written");
        // PNG magic: 89 50 4E 47 0D 0A 1A 0A
        assert!(bytes.len() > 8);
        assert_eq!(
            &bytes[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn transparent_bg_writes_png() {
        let p = tmp_path("transparent");
        run("data", 256, "#000", "transparent", None, "M", false, true, Some(&p)).unwrap();
        let bytes = fs::read(&p).expect("PNG was not written");
        assert_eq!(
            &bytes[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
        // The first pixel should be transparent (alpha = 0).
        let img = image::open(&p).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn coloured_fg_bg() {
        let p = tmp_path("coloured");
        run("xyz", 256, "#ff0000", "#00ff00", None, "M", false, true, Some(&p)).unwrap();
        let img = image::open(&p).unwrap().to_rgba8();
        // The corner of the quiet zone should be the background colour.
        let corner = img.get_pixel(0, 0);
        assert_eq!(corner.0, [0, 255, 0, 255]);
        let _ = fs::remove_file(&p);
    }
}
