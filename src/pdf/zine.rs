//! Zine: single-sheet 8-page mini-zine imposer.
//!
//! Takes exactly 8 images and arranges them on one landscape sheet (4 columns × 2 rows).
//! After printing, folding along the centre creases and cutting the middle horizontal slit,
//! the resulting booklet reads pages 1..8 in order.
//!
//! Layout on sheet (looking at the printed sheet, top-up):
//! ```text
//!   [ p5↻ ][ p4↻ ][ p3↻ ][ p2↻ ]   top row, rotated 180°
//!   [ p6  ][ p7  ][ p8  ][ p1  ]   bottom row, upright
//! ```

use crate::error::Error;
use image::{GenericImageView, imageops::FilterType};
use printpdf::{
    ColorBits, ColorSpace, Image, ImageTransform, ImageXObject, Mm, PdfDocument, Px,
};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

/// (column 0..3, row 0..1, rotation_degrees) indexed by page index (0=page1..7=page8).
const ZINE_LAYOUT: [(u32, u32, u32); 8] = [
    // page 1 → bottom-right (col 3, row 1), upright
    (3, 1, 0),
    // page 2 → top-right-most (col 3, row 0), rotated 180
    (3, 0, 180),
    // page 3 → (col 2, row 0), 180
    (2, 0, 180),
    // page 4 → (col 1, row 0), 180
    (1, 0, 180),
    // page 5 → (col 0, row 0), 180
    (0, 0, 180),
    // page 6 → (col 0, row 1), upright
    (0, 1, 0),
    // page 7 → (col 1, row 1), upright
    (1, 1, 0),
    // page 8 → (col 2, row 1), upright
    (2, 1, 0),
];

/// Crop+resize an image to fill `target_w × target_h` pixels (cover scaling).
/// Returns RGB8 pixel bytes.
fn cover_to_rgb8(img: &image::DynamicImage, target_w: u32, target_h: u32) -> Vec<u8> {
    let (src_w, src_h) = img.dimensions();
    let src_aspect = src_w as f64 / src_h as f64;
    let tgt_aspect = target_w as f64 / target_h as f64;

    // Compute crop window in source pixels to match target aspect ratio.
    let (crop_w, crop_h) = if src_aspect > tgt_aspect {
        // Source is wider → crop sides.
        let crop_w = (src_h as f64 * tgt_aspect).round() as u32;
        (crop_w.min(src_w), src_h)
    } else {
        // Source is taller → crop top/bottom.
        let crop_h = (src_w as f64 / tgt_aspect).round() as u32;
        (src_w, crop_h.min(src_h))
    };
    let crop_x = (src_w.saturating_sub(crop_w)) / 2;
    let crop_y = (src_h.saturating_sub(crop_h)) / 2;
    let cropped = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
    let resized = cropped.resize_exact(target_w, target_h, FilterType::Lanczos3);
    resized.to_rgb8().into_raw()
}

/// Generate a single-sheet zine PDF.
pub fn run(
    images: &[PathBuf],
    paper: &str,
    dpi: f64,
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if images.len() != 8 {
        return Err(Error::Usage(format!(
            "zine requires exactly 8 images (got {})",
            images.len()
        )));
    }
    if !(36.0..=2400.0).contains(&dpi) {
        return Err(Error::Usage(format!(
            "dpi {dpi} out of range (36..2400)"
        )));
    }

    // Validate paper exists.
    let (paper_w_mm, paper_h_mm) = crate::text::paper::lookup_mm(paper)?;
    // Sheet is landscape: longer edge horizontal.
    let sheet_w_mm = paper_w_mm.max(paper_h_mm);
    let sheet_h_mm = paper_w_mm.min(paper_h_mm);
    // Cell sizes in mm.
    let cell_w_mm = sheet_w_mm / 4.0;
    let cell_h_mm = sheet_h_mm / 2.0;
    // Cell sizes in pixels at the chosen DPI.
    let cell_w_px = ((cell_w_mm / 25.4) * dpi).round() as u32;
    let cell_h_px = ((cell_h_mm / 25.4) * dpi).round() as u32;
    if cell_w_px == 0 || cell_h_px == 0 {
        return Err(Error::Usage("cell size resolves to zero pixels".into()));
    }

    // Load + rescale all 8 images.
    let mut cells: Vec<Vec<u8>> = Vec::with_capacity(8);
    for (i, path) in images.iter().enumerate() {
        let img = image::open(path)
            .map_err(|e| Error::Input(format!("failed to read image {}: {e}", path.display())))?;
        let (placement_col, placement_row, rotation) = ZINE_LAYOUT[i];
        let mut rgb = cover_to_rgb8(&img, cell_w_px, cell_h_px);
        if rotation == 180 {
            // Rotate in place: reverse the 3-byte triples globally.
            let total = (cell_w_px * cell_h_px) as usize;
            // Treat as Vec<[u8;3]>
            let mut rotated = vec![0u8; rgb.len()];
            for src_idx in 0..total {
                let dst_idx = total - 1 - src_idx;
                rotated[dst_idx * 3] = rgb[src_idx * 3];
                rotated[dst_idx * 3 + 1] = rgb[src_idx * 3 + 1];
                rotated[dst_idx * 3 + 2] = rgb[src_idx * 3 + 2];
            }
            rgb = rotated;
        }
        let _ = (placement_col, placement_row); // (used below in placement loop)
        cells.push(rgb);
    }

    // Build the PDF.
    let (doc, page_idx, layer_idx) = PdfDocument::new(
        "delphi zine",
        Mm(sheet_w_mm as f32),
        Mm(sheet_h_mm as f32),
        "Layer 1",
    );
    let layer = doc.get_page(page_idx).get_layer(layer_idx);

    for (i, rgb_bytes) in cells.into_iter().enumerate() {
        let (col, row, _rot) = ZINE_LAYOUT[i];
        // PDF coordinates: origin bottom-left. Row 0 = top, Row 1 = bottom.
        let cell_x_mm = col as f64 * cell_w_mm;
        // y from bottom: for row 1 (bottom row), the image base is 0; for row 0 (top), base is cell_h_mm.
        let cell_y_from_bottom_mm = if row == 0 { cell_h_mm } else { 0.0 };

        let image_xo = ImageXObject {
            width: Px(cell_w_px as usize),
            height: Px(cell_h_px as usize),
            color_space: ColorSpace::Rgb,
            bits_per_component: ColorBits::Bit8,
            interpolate: true,
            image_data: rgb_bytes,
            image_filter: None,
            smask: None,
            clipping_bbox: None,
        };
        let img = Image::from(image_xo);
        img.add_to_layer(
            layer.clone(),
            ImageTransform {
                translate_x: Some(Mm(cell_x_mm as f32)),
                translate_y: Some(Mm(cell_y_from_bottom_mm as f32)),
                dpi: Some(dpi as f32),
                ..Default::default()
            },
        );
    }

    // Decide output path.
    let out_path: PathBuf = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("zine.pdf"));

    // Write.
    let file = std::fs::File::create(&out_path)
        .map_err(|e| Error::Processing(format!("create {}: {e}", out_path.display())))?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer)
        .map_err(|e| Error::Processing(format!("PDF save failed: {e}")))?;

    if json {
        let v = serde_json::json!({
            "output": out_path.display().to_string(),
            "paper": paper,
            "sheet_mm": [round1(sheet_w_mm), round1(sheet_h_mm)],
            "cell_mm": [round1(cell_w_mm), round1(cell_h_mm)],
            "cell_px": [cell_w_px, cell_h_px],
            "dpi": dpi,
            "pages": 8,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    } else if !quiet {
        println!("{}", out_path.display());
    }

    Ok(())
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb};

    fn make_dummy_image(path: &Path, w: u32, h: u32, colour: [u8; 3]) {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(w, h, |_, _| Rgb(colour));
        img.save(path).unwrap();
    }

    #[test]
    fn cover_to_rgb8_correct_size() {
        let img = image::DynamicImage::ImageRgb8(
            ImageBuffer::from_fn(100, 200, |_, _| Rgb([10u8, 20, 30])),
        );
        let rgb = cover_to_rgb8(&img, 50, 50);
        assert_eq!(rgb.len(), 50 * 50 * 3);
    }

    #[test]
    fn zine_errors_on_wrong_image_count() {
        let r = run(&[], "a4", 72.0, false, true, None);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn zine_errors_on_unknown_paper() {
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let mut paths: Vec<PathBuf> = Vec::new();
        for i in 0..8 {
            let p = tmpdir.join(format!("delphi_zine_paper_{pid}_{i}.png"));
            make_dummy_image(&p, 8, 8, [(i * 30) as u8, 100, 200]);
            paths.push(p);
        }
        let r = run(&paths, "Z99", 72.0, false, true, None);
        for p in &paths {
            let _ = std::fs::remove_file(p);
        }
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn zine_writes_valid_pdf() {
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let mut paths: Vec<PathBuf> = Vec::new();
        for i in 0..8 {
            let p = tmpdir.join(format!("delphi_zine_img_{pid}_{i}.png"));
            // 50x50 single-colour pngs — small enough to keep test fast at 72dpi
            make_dummy_image(&p, 50, 50, [(i * 30) as u8, 100, 200]);
            paths.push(p);
        }
        let out = tmpdir.join(format!("delphi_zine_out_{pid}.pdf"));
        // Use low DPI to keep the test light.
        let r = run(&paths, "a4", 72.0, false, true, Some(&out));
        for p in &paths {
            let _ = std::fs::remove_file(p);
        }
        r.unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        // Should be re-readable by lopdf.
        let _ = lopdf::Document::load(&out).expect("lopdf must reload the output");
        let _ = std::fs::remove_file(&out);
    }
}
