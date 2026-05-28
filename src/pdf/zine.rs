//! Zine: single-sheet folded-zine imposer.
//!
//! Arranges images on one landscape sheet for a chosen fold template, ready to
//! print, fold, and (for the mini-8) cut. The sheet is always landscape
//! (long edge horizontal): `sheet_w = max(paper_w, paper_h)`,
//! `sheet_h = min(paper_w, paper_h)`.
//!
//! ## Folds
//!
//! **mini-8** — the classic 8-page mini-zine on a 4×2 grid, printed
//! single-sided. After folding along the centre creases and cutting the middle
//! horizontal slit, it reads pages 1..8 in order. Layout (top-up):
//! ```text
//!   [ p5↻ ][ p4↻ ][ p3↻ ][ p2↻ ]   top row, rotated 180°
//!   [ p6  ][ p7  ][ p8  ][ p1  ]   bottom row, upright
//! ```
//!
//! **accordion** — a zig-zag concertina on a 1×N grid (N ∈ {4,6,8}), no cut.
//! Single-sided produces a fold-out strip (pages 1..N left to right).
//! Double-sided produces a continuous booklet: front carries pages 1..N and the
//! back carries pages N+1..2N (leftmost back = N+1), printed two-sided with a
//! **short-edge** flip so reading front-then-back stays continuous and upright.

use crate::error::Error;
use image::{GenericImageView, imageops::FilterType};
use printpdf::{
    ColorBits, ColorSpace, Image, ImageTransform, ImageXObject, Mm, PdfDocument, Px,
};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

/// One image placed in a grid cell on one printed side of the sheet.
#[derive(Clone, Copy, Debug)]
struct Placement {
    /// 1-indexed page/slot number (selects `images[page - 1]`).
    page: u32,
    /// Column index (0-based, left to right).
    col: u32,
    /// Row index (0-based, top to bottom).
    row: u32,
    /// Rotation in degrees applied when drawing (0 or 180).
    rotation: u32,
}

/// A fold template. `MiniEight` is the long-standing default and is preserved
/// byte-for-byte; `Accordion` is configurable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fold {
    MiniEight,
    Accordion { panels: u32, double_sided: bool },
}

/// Classic 8-page mini-zine placements (4 columns × 2 rows, single side).
///
/// Identical to the long-standing `ZINE_LAYOUT`: top row rotated 180° carries
/// pages 5,4,3,2; bottom row upright carries pages 6,7,8,1.
const MINI8_PLACEMENTS: [Placement; 8] = [
    Placement { page: 5, col: 0, row: 0, rotation: 180 },
    Placement { page: 4, col: 1, row: 0, rotation: 180 },
    Placement { page: 3, col: 2, row: 0, rotation: 180 },
    Placement { page: 2, col: 3, row: 0, rotation: 180 },
    Placement { page: 6, col: 0, row: 1, rotation: 0 },
    Placement { page: 7, col: 1, row: 1, rotation: 0 },
    Placement { page: 8, col: 2, row: 1, rotation: 0 },
    Placement { page: 1, col: 3, row: 1, rotation: 0 },
];

impl Fold {
    /// Short, round-trippable identifier (matches the `--fold` flag value and
    /// the JSON `"fold"` field).
    fn id(&self) -> &'static str {
        match self {
            Fold::MiniEight => "mini8",
            Fold::Accordion { .. } => "accordion",
        }
    }

    /// Grid dimensions on the landscape sheet: `(cols, rows)`.
    fn cols_rows(&self) -> (u32, u32) {
        match self {
            Fold::MiniEight => (4, 2),
            Fold::Accordion { panels, .. } => (*panels, 1),
        }
    }

    /// Number of printed sides (PDF pages): 1 (single-sided) or 2 (double).
    fn side_count(&self) -> u32 {
        match self {
            Fold::MiniEight => 1,
            Fold::Accordion { double_sided, .. } => {
                if *double_sided { 2 } else { 1 }
            }
        }
    }

    /// Total page slots the user must supply, across all sides.
    fn page_count(&self) -> u32 {
        match self {
            Fold::MiniEight => 8,
            Fold::Accordion { panels, double_sided } => {
                if *double_sided { panels * 2 } else { *panels }
            }
        }
    }

    /// Placements for each printed side, one inner `Vec` per PDF page (front
    /// then, if double-sided, back).
    fn sides(&self) -> Vec<Vec<Placement>> {
        match self {
            Fold::MiniEight => vec![MINI8_PLACEMENTS.to_vec()],
            Fold::Accordion { panels, double_sided } => {
                let n = *panels;
                // Front: pages 1..N, left to right, upright.
                let front: Vec<Placement> = (0..n)
                    .map(|c| Placement { page: c + 1, col: c, row: 0, rotation: 0 })
                    .collect();
                let mut sides = vec![front];
                if *double_sided {
                    // Back: pages N+1..2N, left to right, upright (no column
                    // reversal — the short-edge flip is physical only).
                    let back: Vec<Placement> = (0..n)
                        .map(|c| Placement { page: n + 1 + c, col: c, row: 0, rotation: 0 })
                        .collect();
                    sides.push(back);
                }
                sides
            }
        }
    }
}

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

/// Rotate `rgb` (RGB8, `w × h`) by 180° in place by reversing the pixel order.
fn rotate180_rgb8(rgb: &[u8], w: u32, h: u32) -> Vec<u8> {
    let total = (w * h) as usize;
    let mut rotated = vec![0u8; rgb.len()];
    for src_idx in 0..total {
        let dst_idx = total - 1 - src_idx;
        rotated[dst_idx * 3] = rgb[src_idx * 3];
        rotated[dst_idx * 3 + 1] = rgb[src_idx * 3 + 1];
        rotated[dst_idx * 3 + 2] = rgb[src_idx * 3 + 2];
    }
    rotated
}

/// Generate a single-sheet zine PDF for the chosen `fold`.
pub fn run(
    images: &[PathBuf],
    fold: Fold,
    paper: &str,
    dpi: f64,
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    // Validate fold configuration first so the error is precise.
    if let Fold::Accordion { panels, .. } = fold {
        if !matches!(panels, 4 | 6 | 8) {
            return Err(Error::Usage(format!(
                "accordion panels must be 4, 6, or 8 (got {panels})"
            )));
        }
    }

    let required = fold.page_count() as usize;
    if images.len() != required {
        return Err(Error::Usage(format!(
            "{} fold requires exactly {} images (got {})",
            fold.id(),
            required,
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

    // Grid + cell sizes (generalised: this reproduces the mini-8 4×2 grid exactly).
    let (cols, rows) = fold.cols_rows();
    let cell_w_mm = sheet_w_mm / cols as f64;
    let cell_h_mm = sheet_h_mm / rows as f64;
    let cell_w_px = ((cell_w_mm / 25.4) * dpi).round() as u32;
    let cell_h_px = ((cell_h_mm / 25.4) * dpi).round() as u32;
    if cell_w_px == 0 || cell_h_px == 0 {
        return Err(Error::Usage("cell size resolves to zero pixels".into()));
    }

    let sides = fold.sides();

    // Build the PDF (first side on the initial page; further sides via add_page).
    let (doc, page_idx, layer_idx) = PdfDocument::new(
        "delphi zine",
        Mm(sheet_w_mm as f32),
        Mm(sheet_h_mm as f32),
        "Layer 1",
    );

    for (side_idx, placements) in sides.iter().enumerate() {
        let layer = if side_idx == 0 {
            doc.get_page(page_idx).get_layer(layer_idx)
        } else {
            let (p, l) = doc.add_page(
                Mm(sheet_w_mm as f32),
                Mm(sheet_h_mm as f32),
                "Layer 1",
            );
            doc.get_page(p).get_layer(l)
        };

        for placement in placements {
            let img = image::open(&images[(placement.page - 1) as usize]).map_err(|e| {
                Error::Input(format!(
                    "failed to read image {}: {e}",
                    images[(placement.page - 1) as usize].display()
                ))
            })?;
            let mut rgb = cover_to_rgb8(&img, cell_w_px, cell_h_px);
            if placement.rotation == 180 {
                rgb = rotate180_rgb8(&rgb, cell_w_px, cell_h_px);
            }

            // PDF coordinates: origin bottom-left. Row 0 = top row.
            let cell_x_mm = placement.col as f64 * cell_w_mm;
            let cell_y_from_bottom_mm = (rows - 1 - placement.row) as f64 * cell_h_mm;

            let image_xo = ImageXObject {
                width: Px(cell_w_px as usize),
                height: Px(cell_h_px as usize),
                color_space: ColorSpace::Rgb,
                bits_per_component: ColorBits::Bit8,
                interpolate: true,
                image_data: rgb,
                image_filter: None,
                smask: None,
                clipping_bbox: None,
            };
            let pdf_img = Image::from(image_xo);
            pdf_img.add_to_layer(
                layer.clone(),
                ImageTransform {
                    translate_x: Some(Mm(cell_x_mm as f32)),
                    translate_y: Some(Mm(cell_y_from_bottom_mm as f32)),
                    dpi: Some(dpi as f32),
                    ..Default::default()
                },
            );
        }
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
            "fold": fold.id(),
            "paper": paper,
            "sheet_mm": [round1(sheet_w_mm), round1(sheet_h_mm)],
            "cell_mm": [round1(cell_w_mm), round1(cell_h_mm)],
            "cell_px": [cell_w_px, cell_h_px],
            "dpi": dpi,
            "sides": fold.side_count(),
            "pages": fold.page_count(),
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

    /// Write `n` small single-colour PNGs into the temp dir, returning paths.
    fn dummy_set(tag: &str, n: usize) -> Vec<PathBuf> {
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let mut paths = Vec::with_capacity(n);
        for i in 0..n {
            let p = tmpdir.join(format!("delphi_zine_{tag}_{pid}_{i}.png"));
            make_dummy_image(&p, 50, 50, [(i as u32 * 30) as u8, 100, 200]);
            paths.push(p);
        }
        paths
    }

    fn cleanup(paths: &[PathBuf]) {
        for p in paths {
            let _ = std::fs::remove_file(p);
        }
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
        let r = run(&[], Fold::MiniEight, "a4", 72.0, false, true, None);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn accordion_errors_on_bad_panels() {
        let paths = dummy_set("badpanel", 5);
        let r = run(
            &paths,
            Fold::Accordion { panels: 5, double_sided: false },
            "a4",
            72.0,
            false,
            true,
            None,
        );
        cleanup(&paths);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn zine_errors_on_unknown_paper() {
        let paths = dummy_set("paper", 8);
        let r = run(&paths, Fold::MiniEight, "Z99", 72.0, false, true, None);
        cleanup(&paths);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn mini8_writes_valid_pdf() {
        let paths = dummy_set("mini8", 8);
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let out = tmpdir.join(format!("delphi_zine_mini8_out_{pid}.pdf"));
        let r = run(&paths, Fold::MiniEight, "a4", 72.0, false, true, Some(&out));
        cleanup(&paths);
        r.unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        let doc = lopdf::Document::load(&out).expect("lopdf must reload the output");
        assert_eq!(doc.get_pages().len(), 1, "mini-8 is a single-page PDF");
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn accordion_single_sided_writes_one_page() {
        // 6-panel single-sided → 6 images → one PDF page.
        let paths = dummy_set("acc6s", 6);
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let out = tmpdir.join(format!("delphi_zine_acc6s_out_{pid}.pdf"));
        let r = run(
            &paths,
            Fold::Accordion { panels: 6, double_sided: false },
            "a4",
            72.0,
            false,
            true,
            Some(&out),
        );
        cleanup(&paths);
        r.unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        let doc = lopdf::Document::load(&out).expect("lopdf must reload the output");
        assert_eq!(doc.get_pages().len(), 1, "single-sided accordion is one page");
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn accordion_double_sided_writes_two_pages() {
        // 6-panel double-sided → 12 images → two PDF pages.
        let paths = dummy_set("acc6d", 12);
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let out = tmpdir.join(format!("delphi_zine_acc6d_out_{pid}.pdf"));
        let r = run(
            &paths,
            Fold::Accordion { panels: 6, double_sided: true },
            "a4",
            72.0,
            false,
            true,
            Some(&out),
        );
        cleanup(&paths);
        r.unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        let doc = lopdf::Document::load(&out).expect("lopdf must reload the output");
        assert_eq!(doc.get_pages().len(), 2, "double-sided accordion is two pages");
        let _ = std::fs::remove_file(&out);
    }
}
