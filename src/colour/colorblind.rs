use crate::colour::Colour;
use crate::error::Error;
use crate::image_tools::{derive_output, open_image};
use crate::output::{colour_block, is_tty};
use image::RgbaImage;
use serde_json::json;
use std::path::{Path, PathBuf};

/// 3x3 transformation matrices in row-major order.
/// Applied in gamma-encoded sRGB (0-255) space to match the reference web tool.
/// Source: Machado, Oliveira and Fernandes (2009) approximations.
const MATRICES: &[(&str, [[f32; 3]; 3])] = &[
    ("normal", [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]),
    ("protanopia", [
        [0.567, 0.433, 0.0],
        [0.558, 0.442, 0.0],
        [0.0, 0.242, 0.758],
    ]),
    ("deuteranopia", [
        [0.625, 0.375, 0.0],
        [0.7, 0.3, 0.0],
        [0.0, 0.3, 0.7],
    ]),
    ("tritanopia", [
        [0.95, 0.05, 0.0],
        [0.0, 0.433, 0.567],
        [0.0, 0.475, 0.525],
    ]),
    ("protanomaly", [
        [0.817, 0.183, 0.0],
        [0.333, 0.667, 0.0],
        [0.0, 0.125, 0.875],
    ]),
    ("deuteranomaly", [
        [0.8, 0.2, 0.0],
        [0.258, 0.742, 0.0],
        [0.0, 0.142, 0.858],
    ]),
    ("tritanomaly", [
        [0.967, 0.033, 0.0],
        [0.0, 0.733, 0.267],
        [0.0, 0.183, 0.817],
    ]),
    ("achromatopsia", [
        [0.299, 0.587, 0.114],
        [0.299, 0.587, 0.114],
        [0.299, 0.587, 0.114],
    ]),
    ("achromatomaly", [
        [0.618, 0.320, 0.062],
        [0.163, 0.775, 0.062],
        [0.163, 0.320, 0.516],
    ]),
];

fn matrix_for(name: &str) -> Result<[[f32; 3]; 3], Error> {
    for (n, m) in MATRICES {
        if n.eq_ignore_ascii_case(name) {
            return Ok(*m);
        }
    }
    let valid: Vec<&str> = MATRICES.iter().map(|(n, _)| *n).collect();
    Err(Error::Usage(format!(
        "unknown colorblind type: {name} (valid: {})",
        valid.join(", ")
    )))
}

#[inline]
fn apply_matrix(r: u8, g: u8, b: u8, m: &[[f32; 3]; 3]) -> (u8, u8, u8) {
    let r = r as f32;
    let g = g as f32;
    let b = b as f32;
    let nr = (m[0][0] * r + m[0][1] * g + m[0][2] * b).round().clamp(0.0, 255.0) as u8;
    let ng = (m[1][0] * r + m[1][1] * g + m[1][2] * b).round().clamp(0.0, 255.0) as u8;
    let nb = (m[2][0] * r + m[2][1] * g + m[2][2] * b).round().clamp(0.0, 255.0) as u8;
    (nr, ng, nb)
}

pub fn run(
    input: Option<&str>,
    cb_type: &str,
    colour_mode: bool,
    json: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    let input =
        input.ok_or_else(|| Error::Usage("colorblind: missing input (colour or image path)".into()))?;
    let m = matrix_for(cb_type)?;

    if colour_mode {
        let c = Colour::parse(input)?;
        let (r, g, b) = c.to_u8();
        let (nr, ng, nb) = apply_matrix(r, g, b, &m);
        let hex = Colour::from_u8(nr, ng, nb).to_hex();
        if json {
            let obj = json!({
                "input": input,
                "type": cb_type,
                "hex": hex,
                "rgb": [nr, ng, nb],
                "original_hex": c.to_hex(),
            });
            println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        } else if is_tty() {
            println!("{}  {}", colour_block(nr, ng, nb), hex);
        } else {
            println!("{hex}");
        }
        return Ok(());
    }

    // Image mode
    let path = PathBuf::from(input);
    if !path.exists() {
        return Err(Error::Input(format!("file not found: {}", path.display())));
    }
    let img = open_image(&path)?.to_rgba8();

    let (w, h) = (img.width(), img.height());
    let mut out: RgbaImage = RgbaImage::new(w, h);
    for (in_p, out_p) in img.pixels().zip(out.pixels_mut()) {
        let [r, g, b, a] = in_p.0;
        let (nr, ng, nb) = apply_matrix(r, g, b, &m);
        out_p.0 = [nr, ng, nb, a];
    }

    let derived = derive_output(&path, "cb", None);
    let out_path = match output {
        Some(p) => {
            if p.is_dir() {
                p.join(derived.file_name().unwrap())
            } else {
                p.to_path_buf()
            }
        }
        None => derived,
    };

    out.save(&out_path)
        .map_err(|e| Error::Processing(format!("could not save {}: {e}", out_path.display())))?;

    if json {
        let obj = json!({
            "input": path.display().to_string(),
            "output": out_path.display().to_string(),
            "type": cb_type,
            "width": w,
            "height": h,
        });
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
    } else {
        println!("{} -> {}", path.display(), out_path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    #[test]
    fn normal_matrix_identity() {
        let m = matrix_for("normal").unwrap();
        let (r, g, b) = apply_matrix(123, 200, 50, &m);
        assert_eq!((r, g, b), (123, 200, 50));
    }

    #[test]
    fn achromatopsia_is_grayscale() {
        let m = matrix_for("achromatopsia").unwrap();
        let (r, g, b) = apply_matrix(255, 0, 0, &m);
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn deuteranopia_red() {
        // Red light should become something brownish/yellow (g > 0 even though we sent pure red).
        let m = matrix_for("deuteranopia").unwrap();
        let (r, g, b) = apply_matrix(255, 0, 0, &m);
        // protanopia/deuteranopia move red toward yellow → G > 0
        assert!(g > 100, "expected significant green channel, got {g}");
        assert!(b == 0, "expected 0 blue, got {b}");
        // R should still be substantial
        assert!(r > 100);
    }

    #[test]
    fn rejects_unknown_type() {
        assert!(matrix_for("not-a-real-type").is_err());
    }

    #[test]
    fn image_dimensions_preserved() {
        // synthetic 4x4 RGBA, run apply_matrix manually
        let mut img = RgbaImage::new(4, 4);
        for p in img.pixels_mut() {
            p.0 = [200, 100, 50, 255];
        }
        let m = matrix_for("deuteranopia").unwrap();
        let mut out = RgbaImage::new(img.width(), img.height());
        for (i, o) in img.pixels().zip(out.pixels_mut()) {
            let (r, g, b) = apply_matrix(i.0[0], i.0[1], i.0[2], &m);
            o.0 = [r, g, b, i.0[3]];
        }
        assert_eq!(out.width(), 4);
        assert_eq!(out.height(), 4);
    }

    #[test]
    fn case_insensitive() {
        assert!(matrix_for("DEUTERANOPIA").is_ok());
        assert!(matrix_for("Tritanopia").is_ok());
    }
}
