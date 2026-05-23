use crate::error::Error;
use barcoders::sym::{
    codabar::Codabar, code128::Code128, code39::Code39, code93::Code93, ean13::EAN13, ean8::EAN8,
    tf::TF,
};
use image::{ImageBuffer, Rgba};
use serde_json::json;
use std::path::{Path, PathBuf};

#[allow(clippy::too_many_arguments)]
pub fn run(
    data: &str,
    format: &str,
    height: u32,
    scale: u32,
    text: bool,
    json_out: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if data.is_empty() {
        return Err(Error::Usage("barcode: empty data".into()));
    }
    if height == 0 {
        return Err(Error::Usage("barcode: --height must be > 0".into()));
    }
    if scale == 0 {
        return Err(Error::Usage("barcode: --scale must be > 0".into()));
    }
    if text {
        // The `--text` overlay would require bundling a glyph font and rendering it,
        // which isn't viable without a font dependency. The spec authorises omitting
        // this with documentation.
        eprintln!("barcode: --text is not supported in this build (no bundled font); ignoring.");
    }

    let fmt = format.trim().to_ascii_lowercase();
    let bits: Vec<u8> = match fmt.as_str() {
        "ean13" => EAN13::new(data)
            .map_err(|e| Error::Input(format!("barcode: ean13: {e}")))?
            .encode(),
        "ean8" => EAN8::new(data)
            .map_err(|e| Error::Input(format!("barcode: ean8: {e}")))?
            .encode(),
        "upca" => {
            // UPC-A is EAN-13 with a leading 0. Accept 11 (calc check digit) or 12 digits.
            if !data.chars().all(|c| c.is_ascii_digit()) {
                return Err(Error::Input(
                    "barcode: upca: data must be numeric (11 or 12 digits)".into(),
                ));
            }
            let normalised = match data.len() {
                11 | 12 => format!("0{}", data),
                _ => {
                    return Err(Error::Input(
                        "barcode: upca: data must be 11 or 12 digits".into(),
                    ))
                }
            };
            EAN13::new(normalised)
                .map_err(|e| Error::Input(format!("barcode: upca: {e}")))?
                .encode()
        }
        "code39" => Code39::new(data)
            .map_err(|e| Error::Input(format!("barcode: code39: {e}")))?
            .encode(),
        "code128" => {
            // Code128 requires a leading character-set indicator (À/Ɓ/Ć).
            // If the user didn't supply one, prepend Ɓ (set B — alphanumeric).
            let first = data.chars().next();
            let needs_prefix = !matches!(first, Some('À') | Some('Ɓ') | Some('Ć'));
            let payload: String = if needs_prefix {
                let mut s = String::with_capacity(data.len() + 2);
                s.push('Ɓ');
                s.push_str(data);
                s
            } else {
                data.to_string()
            };
            Code128::new(payload)
                .map_err(|e| Error::Input(format!("barcode: code128: {e}")))?
                .encode()
        }
        "codabar" => Codabar::new(data)
            .map_err(|e| Error::Input(format!("barcode: codabar: {e}")))?
            .encode(),
        "code93" => Code93::new(data)
            .map_err(|e| Error::Input(format!("barcode: code93: {e}")))?
            .encode(),
        "itf" => TF::interleaved(data)
            .map_err(|e| Error::Input(format!("barcode: itf: {e}")))?
            .encode(),
        other => {
            return Err(Error::Usage(format!(
                "barcode: unsupported format {other} (supported: ean13, ean8, upca, code39, code128, codabar, code93, itf)"
            )));
        }
    };

    if bits.is_empty() {
        return Err(Error::Processing(
            "barcode: encoder produced empty output".into(),
        ));
    }

    // Render bits → PNG.
    let width: u32 = (bits.len() as u32)
        .checked_mul(scale)
        .ok_or_else(|| Error::Processing("barcode: width overflows u32".into()))?;
    let fg = Rgba([0u8, 0, 0, 255]);
    let bg = Rgba([255u8, 255, 255, 255]);

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(width, height, bg);
    for (i, b) in bits.iter().enumerate() {
        if *b == 1 {
            let x0 = (i as u32) * scale;
            for dx in 0..scale {
                let x = x0 + dx;
                for y in 0..height {
                    img.put_pixel(x, y, fg);
                }
            }
        }
    }

    let out_path: PathBuf = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("barcode.png"));
    img.save(&out_path)
        .map_err(|e| Error::Processing(format!("barcode: write {}: {e}", out_path.display())))?;

    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "path": out_path.display().to_string(),
                "format": fmt,
                "width": width,
                "height": height,
            }))
            .unwrap()
        );
    } else if !quiet {
        println!("{}", out_path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "delphi-barcode-test-{}-{}.png",
            std::process::id(),
            name
        ));
        p
    }

    #[test]
    fn rejects_empty_data() {
        let p = tmp_path("empty");
        let err = run("", "code128", 60, 2, false, false, true, Some(&p)).unwrap_err();
        assert!(matches!(err, Error::Usage(_)));
    }

    #[test]
    fn rejects_unknown_format() {
        let p = tmp_path("badfmt");
        let err = run("12345", "datamatrix", 60, 2, false, false, true, Some(&p)).unwrap_err();
        assert!(matches!(err, Error::Usage(_)));
    }

    #[test]
    fn ean13_validates_length() {
        let p = tmp_path("ean13bad");
        // EAN-13 needs 12 or 13 digits; "abc" is neither numeric nor the right length.
        let err = run("abc", "ean13", 60, 2, false, false, true, Some(&p)).unwrap_err();
        assert!(matches!(err, Error::Input(_)));
    }

    #[test]
    fn code128_writes_valid_png() {
        let p = tmp_path("code128");
        run("HELLO", "code128", 60, 2, false, false, true, Some(&p)).unwrap();
        let bytes = fs::read(&p).expect("PNG was not written");
        assert!(bytes.len() > 8);
        // PNG magic header.
        assert_eq!(
            &bytes[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn code39_writes_valid_png() {
        let p = tmp_path("code39");
        run("ABC123", "code39", 60, 2, false, false, true, Some(&p)).unwrap();
        let bytes = fs::read(&p).expect("PNG was not written");
        assert_eq!(
            &bytes[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn upca_normalises_to_ean13() {
        let p = tmp_path("upca");
        // 12 digits (UPC-A with check) is accepted.
        run("012345678905", "upca", 60, 2, false, false, true, Some(&p)).unwrap();
        let bytes = fs::read(&p).expect("PNG was not written");
        assert_eq!(
            &bytes[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
        let _ = fs::remove_file(&p);
    }
}
