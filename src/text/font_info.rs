use crate::error::Error;
use serde_json::json;
use std::path::Path;
use ttf_parser::{name_id, Face};

/// CSS `format(...)` token for a given file extension.
fn css_format(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "ttf" => "truetype",
        "otf" => "opentype",
        "woff" => "woff",
        "woff2" => "woff2",
        _ => "truetype",
    }
}

/// Pull the first decodable name with the given NameId.
fn read_name(face: &Face<'_>, target_id: u16) -> Option<String> {
    for name in face.names() {
        if name.name_id == target_id {
            if let Some(s) = name.to_string() {
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }
    None
}

/// Try to guess a weight from a `font_subfamily` string.
/// Falls back to 400 (normal).
fn guess_weight(subfamily: Option<&str>) -> u16 {
    let s = subfamily.unwrap_or("").to_ascii_lowercase();
    if s.contains("black") || s.contains("heavy") {
        900
    } else if s.contains("extrabold") || s.contains("ultrabold") {
        800
    } else if s.contains("bold") && !s.contains("semi") && !s.contains("demi") {
        700
    } else if s.contains("semibold") || s.contains("demibold") {
        600
    } else if s.contains("medium") {
        500
    } else if s.contains("light") && !s.contains("extra") && !s.contains("ultra") {
        300
    } else if s.contains("extralight") || s.contains("ultralight") {
        200
    } else if s.contains("thin") || s.contains("hairline") {
        100
    } else {
        400
    }
}

pub fn run(font: &Path, as_json: bool) -> Result<(), Error> {
    let ext = font
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // ttf-parser handles TTF/OTF. WOFF/WOFF2 are compressed wrappers; we
    // can't parse them without a decoder. Fall back to filename-based info.
    if ext == "woff" || ext == "woff2" {
        return emit_woff_fallback(font, &ext, as_json);
    }

    let bytes = std::fs::read(font).map_err(|e| {
        Error::Input(format!("font-info: cannot read {}: {e}", font.display()))
    })?;

    let face = Face::parse(&bytes, 0).map_err(|e| {
        Error::Input(format!(
            "font-info: cannot parse {}: {e}",
            font.display()
        ))
    })?;

    let family = read_name(&face, name_id::FAMILY);
    let subfamily = read_name(&face, name_id::SUBFAMILY);
    let full_name = read_name(&face, name_id::FULL_NAME);
    let postscript = read_name(&face, name_id::POST_SCRIPT_NAME);

    let glyph_count = face.number_of_glyphs();
    let upem = face.units_per_em();
    let ascender = face.ascender();
    let descender = face.descender();
    let line_gap = face.line_gap();
    let is_monospaced = face.is_monospaced();
    let is_variable = face.is_variable();
    let axes = face.variation_axes();
    let axis_count = axes.len();

    // Convert font units to px @ 16.
    let scale = if upem == 0 { 0.0 } else { 16.0 / upem as f64 };
    let ascender_px = ascender as f64 * scale;
    let descender_px = descender as f64 * scale;
    let line_gap_px = line_gap as f64 * scale;

    let weight = guess_weight(subfamily.as_deref());
    let format = css_format(&ext);
    let file_name = font
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_else(|| "font.ttf");
    let family_for_css = family
        .clone()
        .or_else(|| full_name.clone())
        .unwrap_or_else(|| file_name.trim_end_matches(&format!(".{ext}")).to_string());

    let css = format!(
        "@font-face {{\n  font-family: \"{family}\";\n  src: url(\"{src}\") format(\"{fmt}\");\n  font-weight: {weight};\n  font-style: normal;\n}}",
        family = family_for_css,
        src = file_name,
        fmt = format,
        weight = weight
    );

    if as_json {
        let v = json!({
            "file": font.display().to_string(),
            "format": format,
            "family": family,
            "subfamily": subfamily,
            "full_name": full_name,
            "postscript_name": postscript,
            "glyphs": glyph_count,
            "units_per_em": upem,
            "ascender": {
                "units": ascender,
                "px_at_16": round2(ascender_px),
            },
            "descender": {
                "units": descender,
                "px_at_16": round2(descender_px),
            },
            "line_gap": {
                "units": line_gap,
                "px_at_16": round2(line_gap_px),
            },
            "monospaced": is_monospaced,
            "variable": is_variable,
            "variation_axes": axis_count,
            "css": css,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    } else {
        println!("File:          {}", font.display());
        println!("Format:        {} ({})", format, ext);
        if let Some(s) = &family {
            println!("Family:        {s}");
        }
        if let Some(s) = &subfamily {
            println!("Subfamily:     {s}");
        }
        if let Some(s) = &full_name {
            println!("Full name:     {s}");
        }
        if let Some(s) = &postscript {
            println!("PostScript:    {s}");
        }
        println!("Glyphs:        {glyph_count}");
        println!("Units/em:      {upem}");
        println!(
            "Ascender:      {ascender} ({:.2}px @ 16)",
            ascender_px
        );
        println!(
            "Descender:     {descender} ({:.2}px @ 16)",
            descender_px
        );
        println!(
            "Line gap:      {line_gap} ({:.2}px @ 16)",
            line_gap_px
        );
        println!("Monospaced:    {is_monospaced}");
        println!("Variable:      {is_variable}");
        if is_variable {
            println!("Variation axes: {axis_count}");
        }
        println!();
        println!("CSS:");
        println!("{css}");
    }

    Ok(())
}

fn emit_woff_fallback(font: &Path, ext: &str, as_json: bool) -> Result<(), Error> {
    let file_name = font
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("font");
    let stem = font
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Font");
    let family = stem.to_string();
    let format = css_format(ext);

    let css = format!(
        "@font-face {{\n  font-family: \"{family}\";\n  src: url(\"{file_name}\") format(\"{format}\");\n  font-weight: 400;\n  font-style: normal;\n}}"
    );

    let msg = format!(
        "font-info: {} files are compressed; ttf-parser does not decode them. \
         Convert to .ttf/.otf for full metadata. Filename-derived info shown below.",
        ext
    );

    if as_json {
        let v = json!({
            "file": font.display().to_string(),
            "format": format,
            "family": family,
            "note": msg,
            "css": css,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    } else {
        eprintln!("warning: {msg}");
        println!("File:    {}", font.display());
        println!("Format:  {format} ({ext})");
        println!("Family:  {family}");
        println!();
        println!("CSS:");
        println!("{css}");
    }

    Ok(())
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_format_recognises_extensions() {
        assert_eq!(css_format("ttf"), "truetype");
        assert_eq!(css_format("TTF"), "truetype");
        assert_eq!(css_format("otf"), "opentype");
        assert_eq!(css_format("woff"), "woff");
        assert_eq!(css_format("woff2"), "woff2");
        assert_eq!(css_format("xyz"), "truetype");
    }

    #[test]
    fn guess_weight_examples() {
        assert_eq!(guess_weight(Some("Regular")), 400);
        assert_eq!(guess_weight(Some("Bold")), 700);
        assert_eq!(guess_weight(Some("Black")), 900);
        assert_eq!(guess_weight(Some("Thin")), 100);
        assert_eq!(guess_weight(Some("ExtraLight")), 200);
        assert_eq!(guess_weight(Some("Light")), 300);
        assert_eq!(guess_weight(Some("Medium")), 500);
        assert_eq!(guess_weight(Some("SemiBold")), 600);
        assert_eq!(guess_weight(Some("ExtraBold")), 800);
        assert_eq!(guess_weight(None), 400);
    }

    #[test]
    fn round2_correctness() {
        assert_eq!(round2(1.234), 1.23);
        assert_eq!(round2(1.235), 1.24);
    }

    #[test]
    fn missing_file_errors() {
        let r = run(Path::new("/no/such/font.ttf"), false);
        assert!(matches!(r, Err(Error::Input(_))));
    }

    #[test]
    fn woff_fallback_does_not_panic() {
        // Synthesise a .woff path that doesn't exist on disk —
        // the woff path never tries to read the file.
        let p = std::env::temp_dir().join("nope.woff");
        run(&p, false).unwrap();
        run(&p, true).unwrap();
    }
}
