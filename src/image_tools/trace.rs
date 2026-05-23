use crate::error::Error;
use crate::image_tools::{derive_output, open_image, resolve_output};
use serde_json::json;
use std::path::Path;
use vtracer::{ColorImage, Config, Preset};

/// Choose a base vtracer Config for the given preset name.
///
/// - `default`   → Preset::Photo  (colour_precision 8, filter_speckle 10)
/// - `detailed`  → tighter colour quantisation, less speckle filtering
/// - `posterize` → coarser colour quantisation, more speckle filtering
pub(crate) fn config_for_preset(preset: &str) -> Result<Config, Error> {
    let mut cfg = match preset {
        "default" => Config::from_preset(Preset::Photo),
        "detailed" => {
            let mut c = Config::from_preset(Preset::Photo);
            c.color_precision = 6;
            c.filter_speckle = 2;
            c.layer_difference = 16;
            c.corner_threshold = 60;
            c
        }
        "posterize" => {
            let mut c = Config::from_preset(Preset::Poster);
            // be more aggressive than the stock Poster preset
            c.color_precision = 3;
            c.filter_speckle = 8;
            c.layer_difference = 32;
            c
        }
        other => {
            return Err(Error::Usage(format!(
                "unknown preset: {other} (expected default, detailed, or posterize)"
            )));
        }
    };

    // sane bounds for downstream maths
    cfg.color_precision = cfg.color_precision.clamp(1, 8);
    Ok(cfg)
}

/// Apply an explicit colour count to a config.
///
/// vtracer expresses colour fidelity via `color_precision` (1-8 bits). We map an
/// approximate colour count to that range: ceil(log2(n)) clamped to 1..=8.
pub(crate) fn apply_colour_override(cfg: &mut Config, colours: u32) {
    let n = colours.max(2);
    // smallest k such that 2^k >= n, then clamp
    let bits = (32 - (n - 1).leading_zeros()) as i32;
    cfg.color_precision = bits.clamp(1, 8);
}

pub fn run(
    image: &Path,
    preset: &str,
    colours: Option<u32>,
    blur: f32,
    json_out: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if !image.is_file() {
        return Err(Error::Input(format!(
            "no such file: {}",
            image.display()
        )));
    }
    if !blur.is_finite() || blur < 0.0 {
        return Err(Error::Usage(format!(
            "--blur must be >= 0 (got {blur})"
        )));
    }

    let mut cfg = config_for_preset(preset)?;
    if let Some(n) = colours {
        apply_colour_override(&mut cfg, n);
    }

    // Load the raster.
    let rgba = open_image(image)?.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);

    // Optional Gaussian pre-blur. `gaussian_blur_f32` panics on sigma <= 0, so guard.
    let rgba = if blur > 0.0 {
        imageproc::filter::gaussian_blur_f32(&rgba, blur)
    } else {
        rgba
    };

    let original_size = std::fs::metadata(image).map(|m| m.len()).unwrap_or(0);

    let color_image = ColorImage {
        pixels: rgba.as_raw().to_vec(),
        width: w,
        height: h,
    };

    let final_precision = cfg.color_precision;
    let svg = vtracer::convert(color_image, cfg)
        .map_err(|e| Error::Processing(format!("trace failed: {e}")))?;
    let svg_string = svg.to_string();

    // Resolve output path.
    let derived = derive_output(image, "traced", Some("svg"));
    let out_path = resolve_output(output, 1, &derived)?;
    std::fs::write(&out_path, &svg_string)
        .map_err(|e| Error::Processing(format!("could not write {}: {e}", out_path.display())))?;

    let svg_size = svg_string.len() as u64;

    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "result": out_path.display().to_string(),
                "original_size": original_size,
                "svg_size": svg_size,
                "preset": preset,
                "colours": final_precision,
            }))
            .unwrap()
        );
    } else if !quiet {
        println!(
            "{} -> {} ({} B -> {} B)",
            image.display(),
            out_path.display(),
            original_size,
            svg_size
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vtracer::ColorMode;

    #[test]
    fn preset_default_is_photo_like() {
        let cfg = config_for_preset("default").unwrap();
        assert!(matches!(cfg.color_mode, ColorMode::Color));
        // Photo preset uses color_precision 8.
        assert_eq!(cfg.color_precision, 8);
    }

    #[test]
    fn preset_detailed_is_tighter() {
        let cfg = config_for_preset("detailed").unwrap();
        assert!(matches!(cfg.color_mode, ColorMode::Color));
        assert!(cfg.color_precision < 8);
        assert!(cfg.filter_speckle <= 4);
    }

    #[test]
    fn preset_posterize_is_coarser() {
        let detailed = config_for_preset("detailed").unwrap();
        let posterize = config_for_preset("posterize").unwrap();
        assert!(posterize.color_precision < detailed.color_precision);
        assert!(posterize.filter_speckle >= detailed.filter_speckle);
    }

    #[test]
    fn unknown_preset_errors() {
        let err = config_for_preset("rainbow").unwrap_err();
        assert!(matches!(err, Error::Usage(_)));
    }

    #[test]
    fn colour_override_maps_to_precision() {
        let mut cfg = config_for_preset("default").unwrap();
        apply_colour_override(&mut cfg, 4);
        // 4 colours => 2 bits
        assert_eq!(cfg.color_precision, 2);

        apply_colour_override(&mut cfg, 16);
        assert_eq!(cfg.color_precision, 4);

        apply_colour_override(&mut cfg, 256);
        assert_eq!(cfg.color_precision, 8);
    }

    #[test]
    fn colour_override_clamps_huge_values() {
        let mut cfg = config_for_preset("default").unwrap();
        apply_colour_override(&mut cfg, 100_000);
        assert!(cfg.color_precision >= 1 && cfg.color_precision <= 8);
    }

    #[test]
    fn colour_override_handles_one_or_zero() {
        let mut cfg = config_for_preset("default").unwrap();
        apply_colour_override(&mut cfg, 0);
        assert!(cfg.color_precision >= 1);
        apply_colour_override(&mut cfg, 1);
        assert!(cfg.color_precision >= 1);
    }
}
