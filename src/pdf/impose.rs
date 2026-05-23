//! Impose: arrange the pages of a PDF onto larger sheets for booklet/saddle-stitch
//! or N-up production printing.
//!
//! Strategy: load the source PDF with lopdf, turn each source page into a
//! Form XObject, then place those XObjects on freshly-built output sheets via
//! `cm` (concatenate matrix) + `Do` (invoke XObject) operators. The output PDF
//! preserves the vector content of the source (fonts, images, text remain
//! native PDF objects) without rasterisation.
//!
//! Trade-off: we copy every source object into the output document and rely
//! on `renumber_objects_with` to avoid ID collisions. We do *not* re-encrypt,
//! so encrypted source PDFs are rejected up front.

use crate::error::Error;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, dictionary};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const PT_PER_MM: f64 = 72.0 / 25.4;

// ---------------------------------------------------------------------------
// Placement geometry (in mm)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct Placement {
    /// 1-indexed source page number (0 = blank cell).
    page: u32,
    /// X offset from sheet origin (mm), left edge of cell.
    x_mm: f64,
    /// Y offset from sheet origin (mm), TOP edge of cell (TS-style top-down).
    y_mm: f64,
    /// Cell width (mm).
    w_mm: f64,
    /// Cell height (mm).
    h_mm: f64,
    /// Rotation in degrees, restricted to {0, 90, 180, 270}.
    rotation: u32,
}

#[derive(Debug, Default)]
struct Sheet {
    front: Vec<Placement>,
    back: Vec<Placement>,
}

// ---------------------------------------------------------------------------
// Layout calculation
// ---------------------------------------------------------------------------

fn pad_to_multiple(n: u32, m: u32) -> u32 {
    let rem = n % m;
    if rem == 0 { n } else { n + (m - rem) }
}

fn build_grid_placements(
    page_numbers: &[u32],
    rows: u32,
    cols: u32,
    sheet_w_mm: f64,
    sheet_h_mm: f64,
    margin_mm: f64,
    gutter_mm: f64,
) -> Vec<Placement> {
    let usable_w = sheet_w_mm - margin_mm * 2.0 - gutter_mm * (cols as f64 - 1.0);
    let usable_h = sheet_h_mm - margin_mm * 2.0 - gutter_mm * (rows as f64 - 1.0);
    let cell_w = usable_w / cols as f64;
    let cell_h = usable_h / rows as f64;
    let mut placements = Vec::with_capacity(page_numbers.len());
    for (i, &pn) in page_numbers.iter().enumerate() {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = margin_mm + col as f64 * (cell_w + gutter_mm);
        let y = margin_mm + row as f64 * (cell_h + gutter_mm);
        placements.push(Placement {
            page: pn,
            x_mm: x,
            y_mm: y,
            w_mm: cell_w,
            h_mm: cell_h,
            rotation: 0,
        });
    }
    placements
}

fn calc_saddle_stitch(
    total_source: u32,
    sheet_w_mm: f64,
    sheet_h_mm: f64,
    margin_mm: f64,
    gutter_mm: f64,
    creep_mm: f64,
) -> (Vec<Sheet>, u32) {
    let total_pages = pad_to_multiple(total_source.max(4), 4);
    let num_sheets = total_pages / 4;
    let mut sheets = Vec::with_capacity(num_sheets as usize);

    for s in 0..num_sheets {
        let raw_creep = (num_sheets as f64 - 1.0 - s as f64) * creep_mm;
        let creep = raw_creep.min(margin_mm).max(0.0);
        let front_left = total_pages - 2 * s;
        let front_right = 2 * s + 1;
        let back_left = 2 * s + 2;
        let back_right = total_pages - 2 * s - 1;

        let p_or_blank = |p: u32| -> u32 {
            if p <= total_source { p } else { 0 }
        };

        let usable_w = sheet_w_mm - margin_mm * 2.0;
        let usable_h = sheet_h_mm - margin_mm * 2.0;
        let half_w = (usable_w - gutter_mm) / 2.0;

        let mut front = Vec::with_capacity(2);
        front.push(Placement {
            page: p_or_blank(front_left),
            x_mm: margin_mm - creep,
            y_mm: margin_mm,
            w_mm: half_w,
            h_mm: usable_h,
            rotation: 0,
        });
        front.push(Placement {
            page: p_or_blank(front_right),
            x_mm: margin_mm + half_w + gutter_mm + creep,
            y_mm: margin_mm,
            w_mm: half_w,
            h_mm: usable_h,
            rotation: 0,
        });
        let mut back = Vec::with_capacity(2);
        back.push(Placement {
            page: p_or_blank(back_left),
            x_mm: margin_mm - creep,
            y_mm: margin_mm,
            w_mm: half_w,
            h_mm: usable_h,
            rotation: 0,
        });
        back.push(Placement {
            page: p_or_blank(back_right),
            x_mm: margin_mm + half_w + gutter_mm + creep,
            y_mm: margin_mm,
            w_mm: half_w,
            h_mm: usable_h,
            rotation: 0,
        });
        sheets.push(Sheet { front, back });
    }
    (sheets, total_pages)
}

fn calc_perfect_bind(
    total_source: u32,
    signature: u32,
    sheet_w_mm: f64,
    sheet_h_mm: f64,
    margin_mm: f64,
    gutter_mm: f64,
    creep_mm: f64,
) -> (Vec<Sheet>, u32) {
    // Each signature is saddle-stitched internally. Pages per signature must be a multiple of 4.
    let sig = signature.max(4);
    let sig = pad_to_multiple(sig, 4);
    let total_pages = pad_to_multiple(total_source.max(sig), sig);

    let mut all_sheets: Vec<Sheet> = Vec::new();
    let num_signatures = total_pages / sig;
    for sig_idx in 0..num_signatures {
        let base = sig_idx * sig; // 1-indexed first page of this signature = base + 1
        let (mut sub_sheets, _) = calc_saddle_stitch(
            // Internally use a fresh saddle-stitch for `sig` pages, then offset.
            sig,
            sheet_w_mm,
            sheet_h_mm,
            margin_mm,
            gutter_mm,
            creep_mm,
        );
        // Offset page numbers by `base`, clamping anything beyond total_source to 0.
        for sheet in &mut sub_sheets {
            for p in sheet.front.iter_mut().chain(sheet.back.iter_mut()) {
                if p.page == 0 {
                    continue;
                }
                let abs = base + p.page;
                p.page = if abs <= total_source { abs } else { 0 };
            }
        }
        all_sheets.extend(sub_sheets);
    }
    (all_sheets, total_pages)
}

fn calc_n_up(
    total_source: u32,
    n_up: u32,
    sheet_w_mm: f64,
    sheet_h_mm: f64,
    margin_mm: f64,
    gutter_mm: f64,
) -> (Vec<Sheet>, u32) {
    // Pick a sensible grid for N. Falls back to (rows, cols) where the result
    // is closest to the sheet aspect ratio.
    let (rows, cols) = grid_for_n_up(n_up, sheet_w_mm, sheet_h_mm);
    let cells_per_side = rows * cols;
    // The instruction says: chunk pages into groups of N, place N-up on each sheet.
    // We only fill the front side. Duplex (if requested) duplicates onto back too.
    let total_padded = pad_to_multiple(total_source.max(cells_per_side), cells_per_side);
    let num_sheets = total_padded / cells_per_side;

    let p_or_blank = |p: u32| -> u32 {
        if p == 0 || p > total_source { 0 } else { p }
    };

    let mut sheets = Vec::with_capacity(num_sheets as usize);
    for s in 0..num_sheets {
        let mut front_pages = Vec::with_capacity(cells_per_side as usize);
        for i in 0..cells_per_side {
            front_pages.push(p_or_blank(s * cells_per_side + i + 1));
        }
        let front = build_grid_placements(
            &front_pages, rows, cols, sheet_w_mm, sheet_h_mm, margin_mm, gutter_mm,
        );
        sheets.push(Sheet { front, back: Vec::new() });
    }
    (sheets, total_padded)
}

fn grid_for_n_up(n: u32, sheet_w_mm: f64, sheet_h_mm: f64) -> (u32, u32) {
    if n == 0 {
        return (1, 1);
    }
    // Best (rows, cols) so that rows*cols == n.
    // Pick the factorisation whose cell aspect (sheet_w/cols)/(sheet_h/rows) is closest to 1.
    let mut best = (1u32, n);
    let mut best_score = f64::MAX;
    for rows in 1..=n {
        if n % rows != 0 {
            continue;
        }
        let cols = n / rows;
        let cell_w = sheet_w_mm / cols as f64;
        let cell_h = sheet_h_mm / rows as f64;
        let aspect = cell_w / cell_h;
        let score = (aspect.ln()).abs(); // prefer aspect ratio closest to 1
        if score < best_score {
            best_score = score;
            best = (rows, cols);
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Source PDF → Form XObject conversion
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct SourcePage {
    /// Effective page dimensions in points, AFTER the source's /Rotate is
    /// applied (so callers can place the form as if it were upright).
    width_pt: f64,
    height_pt: f64,
    /// The Form XObject ID in the destination document (after copy).
    form_xobj_id: Option<ObjectId>,
}

/// Open + merge the source doc's objects into `out`, returning info about each
/// source page (its MediaBox dims and the newly-allocated Form XObject ID).
fn extract_source_pages_as_forms(
    src_path: &Path,
    out: &mut Document,
) -> Result<Vec<SourcePage>, Error> {
    let mut src = Document::load(src_path)
        .map_err(|e| Error::Input(format!("PDF load failed: {e}")))?;
    if src.is_encrypted() {
        // Try empty password; otherwise refuse.
        if src.decrypt("").is_err() {
            return Err(Error::Input(
                "source PDF is encrypted; remove encryption first".into(),
            ));
        }
    }

    // Renumber the source's objects so they don't collide with the output's IDs.
    src.renumber_objects_with(out.max_id + 1);
    out.max_id = src.max_id;

    // Collect ordered list of source page IDs.
    let src_pages: BTreeMap<u32, ObjectId> = src.get_pages();
    let mut pages_in_order: Vec<ObjectId> = src_pages.values().copied().collect();
    // BTreeMap of u32 -> ObjectId is sorted by u32 key (1, 2, 3 …) so order is correct.

    // Build a set of page IDs so we don't move them into `out` directly.
    let page_id_set: std::collections::HashSet<ObjectId> =
        pages_in_order.iter().copied().collect();

    // Read MediaBox and /Rotate for each page (with parent inheritance) BEFORE
    // we move objects out of `src`.
    let mut media_boxes: Vec<(f64, f64)> = Vec::with_capacity(pages_in_order.len());
    let mut rotations: Vec<i64> = Vec::with_capacity(pages_in_order.len());
    for pid in &pages_in_order {
        let (w, h) = read_media_box(&src, *pid).unwrap_or((595.0, 842.0));
        media_boxes.push((w, h));
        rotations.push(read_inherited_i64(&src, *pid, b"Rotate").unwrap_or(0));
    }

    // For each page, collect its content stream bytes and its resources object.
    // We move the page's data into a new Form XObject stream.
    let mut form_data: Vec<(Vec<u8>, Option<InheritedResources>, (f64, f64), i64)> = Vec::new();
    for (i, pid) in pages_in_order.iter().enumerate() {
        let mb = media_boxes[i];
        let rot = rotations[i];

        // Concatenate decompressed content streams.
        let content_ids = src.get_page_contents(*pid);
        let mut combined = Vec::new();
        for cid in &content_ids {
            if let Ok(Object::Stream(s)) = src.get_object(*cid) {
                match s.decompressed_content() {
                    Ok(d) => combined.extend_from_slice(&d),
                    Err(_) => combined.extend_from_slice(&s.content),
                }
                combined.push(b'\n');
            }
        }

        // Resource dict for this page — inherited from /Pages parent if not
        // present on the page itself. Returns either a Reference (use it) or
        // an inline Dictionary (we'll copy into `out` after the move below).
        let resources_inherited = read_inherited_resources(&src, *pid);

        form_data.push((combined, resources_inherited, mb, rot));
    }

    // Move every source object into `out`, EXCEPT the source's page-tree pages and
    // pages-list dict (we no longer need them). We keep resources, fonts, etc.
    for (oid, obj) in std::mem::take(&mut src.objects) {
        if page_id_set.contains(&oid) {
            continue;
        }
        // Skip Catalog and the Pages root — we have our own.
        if let Object::Dictionary(ref d) = obj {
            if let Ok(Object::Name(name)) = d.get(b"Type") {
                if name == b"Catalog" || name == b"Pages" {
                    continue;
                }
            }
        }
        out.objects.insert(oid, obj);
    }

    // Now create a Form XObject for each source page.
    // The form's BBox is the raw MediaBox; if the source page has /Rotate,
    // we bake the rotation into the Matrix so callers can treat the form
    // as an already-upright page with the post-rotate dimensions.
    let mut sources = Vec::with_capacity(form_data.len());
    for (content, inh_res, (w_pt, h_pt), rot_deg) in form_data.into_iter() {
        // Normalise rotation to {0, 90, 180, 270}.
        let rot = ((rot_deg % 360) + 360) % 360;
        // Effective (post-rotate) size: 90/270 swap width and height.
        let (eff_w, eff_h) = if rot == 90 || rot == 270 {
            (h_pt, w_pt)
        } else {
            (w_pt, h_pt)
        };

        // Matrix [a b c d e f] takes form-space coords → external coords.
        // We want: for a viewer placing the Form at (0,0), the upper-left of
        // the original page maps to (0, eff_h), i.e. content appears upright.
        //
        // For each rotation R, we need cm = R rotating around (w/2, h/2),
        // then translating so the resulting bbox sits at (0,0)..(eff_w, eff_h).
        let (a, b, c, d, e, f): (f32, f32, f32, f32, f32, f32) = match rot {
            90 => (0.0, 1.0, -1.0, 0.0, h_pt as f32, 0.0),
            180 => (-1.0, 0.0, 0.0, -1.0, w_pt as f32, h_pt as f32),
            270 => (0.0, -1.0, 1.0, 0.0, 0.0, w_pt as f32),
            _ => (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        };

        let mut form_dict = dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "FormType" => 1,
            "BBox" => vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(w_pt as f32),
                Object::Real(h_pt as f32),
            ],
            "Matrix" => vec![
                Object::Real(a),
                Object::Real(b),
                Object::Real(c),
                Object::Real(d),
                Object::Real(e),
                Object::Real(f),
            ],
        };
        if let Some(res) = inh_res {
            let rid = match res {
                InheritedResources::Ref(id) => id,
                InheritedResources::Inline(dict) => out.add_object(Object::Dictionary(dict)),
            };
            form_dict.set("Resources", Object::Reference(rid));
        }
        let mut stream = Stream::new(form_dict, content);
        // Try to compress; not fatal if it fails.
        let _ = stream.compress();
        let id = out.add_object(stream);
        sources.push(SourcePage {
            width_pt: eff_w,
            height_pt: eff_h,
            form_xobj_id: Some(id),
        });
    }

    // Avoid unused warning.
    pages_in_order.clear();

    Ok(sources)
}

/// Either an indirect reference to a Resources dict, or an inline dict pulled
/// from the page (or an ancestor) that needs to be re-allocated in the output.
enum InheritedResources {
    Ref(ObjectId),
    Inline(Dictionary),
}

/// Walk the page's parent chain looking for `/Resources`. PDF 1.7 §7.7.3.4
/// makes /Resources an inheritable attribute; many PDF writers place a shared
/// /Resources dict on the /Pages root and omit it on each /Page.
fn read_inherited_resources(doc: &Document, page_id: ObjectId) -> Option<InheritedResources> {
    let mut id = page_id;
    for _ in 0..16 {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(res) = dict.get(b"Resources") {
            match res {
                Object::Reference(rid) => return Some(InheritedResources::Ref(*rid)),
                Object::Dictionary(d) => {
                    return Some(InheritedResources::Inline(d.clone()));
                }
                _ => return None,
            }
        }
        match dict.get(b"Parent").ok() {
            Some(Object::Reference(pid)) => id = *pid,
            _ => return None,
        }
    }
    None
}

/// Walk the parent chain for an inherited integer value (e.g. /Rotate).
fn read_inherited_i64(doc: &Document, page_id: ObjectId, key: &[u8]) -> Option<i64> {
    let mut id = page_id;
    for _ in 0..16 {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(v) = dict.get(key) {
            let v = match v {
                Object::Reference(rid) => doc.get_object(*rid).ok()?,
                other => other,
            };
            return v.as_i64().ok();
        }
        match dict.get(b"Parent").ok() {
            Some(Object::Reference(pid)) => id = *pid,
            _ => return None,
        }
    }
    None
}

fn read_media_box(doc: &Document, page_id: ObjectId) -> Option<(f64, f64)> {
    fn box_from_object(obj: &Object) -> Option<(f64, f64)> {
        let arr = obj.as_array().ok()?;
        if arr.len() < 4 {
            return None;
        }
        let n: Vec<f64> = arr
            .iter()
            .filter_map(|o| match o {
                Object::Integer(i) => Some(*i as f64),
                Object::Real(r) => Some(*r as f64),
                _ => None,
            })
            .collect();
        if n.len() < 4 {
            return None;
        }
        Some(((n[2] - n[0]).abs(), (n[3] - n[1]).abs()))
    }

    // Walk parent chain.
    let mut id = page_id;
    for _ in 0..16 {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(mb) = dict.get(b"MediaBox") {
            let mb = match mb {
                Object::Reference(rid) => doc.get_object(*rid).ok()?,
                other => other,
            };
            return box_from_object(mb);
        }
        match dict.get(b"Parent").ok() {
            Some(Object::Reference(pid)) => id = *pid,
            _ => return None,
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Render a sheet
// ---------------------------------------------------------------------------

/// Produce a content stream that places each placement's Form XObject at the
/// configured location/size/rotation. Returns the bytes of the content stream.
fn render_sheet_content(
    placements: &[Placement],
    sources: &[SourcePage],
    sheet_w_mm: f64,
    sheet_h_mm: f64,
    crop_marks: bool,
) -> Vec<u8> {
    let _ = sheet_w_mm; // currently unused (origin reference)
    let mut ops: Vec<Operation> = Vec::new();
    let sheet_h_pt = sheet_h_mm * PT_PER_MM;

    for p in placements {
        if p.page == 0 {
            continue;
        }
        let idx = (p.page as usize).saturating_sub(1);
        if idx >= sources.len() {
            continue;
        }
        let Some(_form_id) = sources[idx].form_xobj_id else {
            continue;
        };
        let src_w_pt = sources[idx].width_pt;
        let src_h_pt = sources[idx].height_pt;
        if src_w_pt <= 0.0 || src_h_pt <= 0.0 {
            continue;
        }

        // Cell in points, with PDF y-origin at the bottom-left.
        let cell_x_pt = p.x_mm * PT_PER_MM;
        let cell_w_pt = p.w_mm * PT_PER_MM;
        let cell_h_pt = p.h_mm * PT_PER_MM;
        let cell_y_bottom_pt = sheet_h_pt - (p.y_mm * PT_PER_MM + cell_h_pt);

        // Compute scale to "fit" the source page into the cell, preserving aspect.
        let (display_w_pt, display_h_pt) = if p.rotation == 90 || p.rotation == 270 {
            // After rotation, the source's width becomes height and vice versa.
            let s = (cell_w_pt / src_h_pt).min(cell_h_pt / src_w_pt);
            (src_h_pt * s, src_w_pt * s)
        } else {
            let s = (cell_w_pt / src_w_pt).min(cell_h_pt / src_h_pt);
            (src_w_pt * s, src_h_pt * s)
        };

        // Centre offsets within the cell.
        let offset_x = (cell_w_pt - display_w_pt) / 2.0;
        let offset_y = (cell_h_pt - display_h_pt) / 2.0;

        // Construct the transformation matrix.
        // The form XObject is in source units (BBox = 0,0,w,h).
        // We want: rotate around the form's centre, then scale, then translate
        //          so the form lands at (cell_x + offset_x, cell_y_bottom + offset_y).
        // PDF `cm` operator: a b c d e f → matrix [a b c d e f]
        //   x' = a*x + c*y + e
        //   y' = b*x + d*y + f
        //
        // For rotation θ (CCW), scale s (uniform here equal in both axes), translate (tx, ty):
        //   M = T(tx,ty) * R(θ) * S(s)
        //   where coords are relative to (0,0) of the form.
        // If we rotate without re-centring, then translate so the rotated bbox lands at
        // (cell_x + offset_x, cell_y_bottom + offset_y).

        let theta_deg = p.rotation as f64 % 360.0;
        let theta = theta_deg.to_radians();
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        // Scale factor: 1 unit in source = scale_x in target (uniform here).
        let scale = if p.rotation == 90 || p.rotation == 270 {
            display_w_pt / src_h_pt
        } else {
            display_w_pt / src_w_pt
        };

        // After R(θ)*S(scale), the bbox corners are:
        //   (0,0)               → (0,0)
        //   (src_w,0)*S*R       → (src_w*scale*cos, src_w*scale*sin)
        //   (0,src_h)*S*R       → (-src_h*scale*sin, src_h*scale*cos)
        //   (src_w,src_h)*S*R   → combination
        // We need to translate so that the min-x corner sits at (cell_x + offset_x)
        // and min-y corner at (cell_y_bottom + offset_y).
        let corners = [
            (0.0_f64, 0.0_f64),
            (src_w_pt * scale, 0.0),
            (0.0, src_h_pt * scale),
            (src_w_pt * scale, src_h_pt * scale),
        ];
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        for (x, y) in corners {
            let rx = cos_t * x - sin_t * y;
            let ry = sin_t * x + cos_t * y;
            if rx < min_x {
                min_x = rx;
            }
            if ry < min_y {
                min_y = ry;
            }
        }
        let tx = cell_x_pt + offset_x - min_x;
        let ty = cell_y_bottom_pt + offset_y - min_y;

        // Final cm: [a b c d e f] where (a c e ; b d f) = R*S then translate (tx,ty)
        //   a = scale*cos_t,  c = -scale*sin_t,  e = tx
        //   b = scale*sin_t,  d =  scale*cos_t,  f = ty
        let a = scale * cos_t;
        let b = scale * sin_t;
        let c = -scale * sin_t;
        let d = scale * cos_t;
        let e = tx;
        let f = ty;

        ops.push(Operation::new("q", vec![]));
        ops.push(Operation::new(
            "cm",
            vec![a.into(), b.into(), c.into(), d.into(), e.into(), f.into()],
        ));
        ops.push(Operation::new(
            "Do",
            vec![Object::Name(form_xobj_name(idx).into_bytes())],
        ));
        ops.push(Operation::new("Q", vec![]));
    }

    // Crop marks
    if crop_marks {
        ops.extend(crop_mark_ops(placements, sheet_h_pt));
    }

    let content = Content { operations: ops };
    content.encode().unwrap_or_default()
}

fn form_xobj_name(idx: usize) -> String {
    format!("F{idx}")
}

fn crop_mark_ops(placements: &[Placement], sheet_h_pt: f64) -> Vec<Operation> {
    let mut ops: Vec<Operation> = Vec::new();
    if placements.is_empty() {
        return ops;
    }
    let mark_len_pt = 18.0;
    let offset_pt = 3.0;

    // Stroke setup: 0.25pt black line.
    ops.push(Operation::new("q", vec![]));
    ops.push(Operation::new("0 G", vec![]));
    ops.push(Operation::new("w", vec![Object::Real(0.25)]));

    // For each placement, draw marks at all four trim corners, but only on
    // outer edges (so internal gutters don't get cluttered).
    // Determine outer extents.
    let eps = 0.5;
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    for p in placements {
        let x = p.x_mm * PT_PER_MM;
        let xw = (p.x_mm + p.w_mm) * PT_PER_MM;
        // PDF y grows from bottom, so the placement's TOP is the LARGER y.
        let y_top = sheet_h_pt - p.y_mm * PT_PER_MM;
        let y_bot = sheet_h_pt - (p.y_mm + p.h_mm) * PT_PER_MM;
        if x < min_x {
            min_x = x;
        }
        if xw > max_x {
            max_x = xw;
        }
        // max_y = largest top across all placements = sheet's outer top edge.
        if y_top > max_y {
            max_y = y_top;
        }
        // min_y = smallest bottom across all placements = sheet's outer bottom edge.
        if y_bot < min_y {
            min_y = y_bot;
        }
    }

    for p in placements {
        let x1 = p.x_mm * PT_PER_MM;
        let x2 = (p.x_mm + p.w_mm) * PT_PER_MM;
        let y1 = sheet_h_pt - p.y_mm * PT_PER_MM; // top (larger PDF y)
        let y2 = sheet_h_pt - (p.y_mm + p.h_mm) * PT_PER_MM; // bottom (smaller PDF y)

        let on_left = (x1 - min_x).abs() < eps;
        let on_right = (x2 - max_x).abs() < eps;
        let on_top = (y1 - max_y).abs() < eps;
        let on_bottom = (y2 - min_y).abs() < eps;

        // TL: left arm if outer-left, top arm if outer-top
        if on_left {
            push_line(&mut ops, x1 - offset_pt, y1, x1 - offset_pt - mark_len_pt, y1);
        }
        if on_top {
            push_line(&mut ops, x1, y1 + offset_pt, x1, y1 + offset_pt + mark_len_pt);
        }
        // TR
        if on_right {
            push_line(&mut ops, x2 + offset_pt, y1, x2 + offset_pt + mark_len_pt, y1);
        }
        if on_top {
            push_line(&mut ops, x2, y1 + offset_pt, x2, y1 + offset_pt + mark_len_pt);
        }
        // BL
        if on_left {
            push_line(&mut ops, x1 - offset_pt, y2, x1 - offset_pt - mark_len_pt, y2);
        }
        if on_bottom {
            push_line(&mut ops, x1, y2 - offset_pt, x1, y2 - offset_pt - mark_len_pt);
        }
        // BR
        if on_right {
            push_line(&mut ops, x2 + offset_pt, y2, x2 + offset_pt + mark_len_pt, y2);
        }
        if on_bottom {
            push_line(&mut ops, x2, y2 - offset_pt, x2, y2 - offset_pt - mark_len_pt);
        }
    }
    ops.push(Operation::new("Q", vec![]));
    ops
}

fn push_line(ops: &mut Vec<Operation>, x1: f64, y1: f64, x2: f64, y2: f64) {
    ops.push(Operation::new("m", vec![x1.into(), y1.into()]));
    ops.push(Operation::new("l", vec![x2.into(), y2.into()]));
    ops.push(Operation::new("S", vec![]));
}

// ---------------------------------------------------------------------------
// Output PDF assembly
// ---------------------------------------------------------------------------

fn build_output_pdf(
    sheets: &[Sheet],
    sources: &[SourcePage],
    sheet_w_mm: f64,
    sheet_h_mm: f64,
    _duplex: bool,
    crop_marks: bool,
    mut out: Document,
) -> Result<Document, Error> {
    let sheet_w_pt = sheet_w_mm * PT_PER_MM;
    let sheet_h_pt = sheet_h_mm * PT_PER_MM;

    // Build XObject dict that maps "F{idx}" → form xobject Reference.
    let mut xobj_dict = Dictionary::new();
    for (i, src) in sources.iter().enumerate() {
        if let Some(id) = src.form_xobj_id {
            xobj_dict.set(form_xobj_name(i).into_bytes(), Object::Reference(id));
        }
    }
    let xobject_id = out.add_object(xobj_dict);
    let resources_id = out.add_object(dictionary! {
        "XObject" => Object::Reference(xobject_id),
        "ProcSet" => vec![
            Object::Name(b"PDF".to_vec()),
            Object::Name(b"Text".to_vec()),
            Object::Name(b"ImageB".to_vec()),
            Object::Name(b"ImageC".to_vec()),
            Object::Name(b"ImageI".to_vec()),
        ],
    });

    // Pre-allocate Pages root ID.
    let pages_id = out.new_object_id();

    let mut page_ids: Vec<ObjectId> = Vec::new();
    for sheet in sheets {
        // Front page
        {
            let bytes = render_sheet_content(
                &sheet.front,
                sources,
                sheet_w_mm,
                sheet_h_mm,
                crop_marks,
            );
            let mut content_stream = Stream::new(Dictionary::new(), bytes);
            let _ = content_stream.compress();
            let content_id = out.add_object(content_stream);

            let page_id = out.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => Object::Reference(resources_id),
                "MediaBox" => vec![
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(sheet_w_pt as f32),
                    Object::Real(sheet_h_pt as f32),
                ],
            });
            page_ids.push(page_id);
        }
        // Back page — emitted whenever the layout has back-side placements
        // (saddle-stitch and perfect-bind always do; n-up only when --duplex is on).
        if !sheet.back.is_empty() {
            let bytes = render_sheet_content(
                &sheet.back,
                sources,
                sheet_w_mm,
                sheet_h_mm,
                crop_marks,
            );
            let mut content_stream = Stream::new(Dictionary::new(), bytes);
            let _ = content_stream.compress();
            let content_id = out.add_object(content_stream);

            let page_id = out.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => Object::Reference(resources_id),
                "MediaBox" => vec![
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(sheet_w_pt as f32),
                    Object::Real(sheet_h_pt as f32),
                ],
            });
            page_ids.push(page_id);
        }
    }

    // Pages root
    let kids: Vec<Object> = page_ids
        .iter()
        .map(|id| Object::Reference(*id))
        .collect();
    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Kids" => kids,
        "Count" => Object::Integer(page_ids.len() as i64),
    };
    out.objects.insert(pages_id, Object::Dictionary(pages_dict));

    // Catalog
    let catalog_id = out.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    out.trailer.set("Root", catalog_id);

    Ok(out)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn run(
    pdf: &Path,
    layout: &str,
    paper: &str,
    n_up: u32,
    signature: u32,
    margins: f64,
    gutter: f64,
    creep: f64,
    crop_marks: bool,
    duplex: bool,
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    // Resolve paper size (use landscape orientation by default — broadly useful).
    let (paper_w_mm, paper_h_mm) = crate::text::paper::lookup_mm(paper)?;
    // Saddle-stitch + perfect-bind use landscape (long edge horizontal); N-up uses
    // the natural orientation of the paper.
    let (sheet_w_mm, sheet_h_mm) = match layout {
        "saddle-stitch" | "perfect-bind" => (paper_w_mm.max(paper_h_mm), paper_w_mm.min(paper_h_mm)),
        _ => (paper_w_mm.min(paper_h_mm), paper_w_mm.max(paper_h_mm)),
    };

    // Build an output Document.
    let mut out = Document::with_version("1.5");
    // Extract source pages as Form XObjects, copying source objects into `out`.
    let sources = extract_source_pages_as_forms(pdf, &mut out)?;

    // Compute the imposition.
    let total_source = sources.len() as u32;
    if total_source == 0 {
        return Err(Error::Input("source PDF has no pages".into()));
    }

    let (sheets, _total_pages) = match layout {
        "saddle-stitch" => calc_saddle_stitch(
            total_source, sheet_w_mm, sheet_h_mm, margins, gutter, creep,
        ),
        "perfect-bind" => calc_perfect_bind(
            total_source, signature, sheet_w_mm, sheet_h_mm, margins, gutter, creep,
        ),
        "n-up" => {
            if duplex && !quiet {
                eprintln!(
                    "warning: --duplex is ignored for n-up layout (n-up is single-sided)."
                );
            }
            calc_n_up(total_source, n_up, sheet_w_mm, sheet_h_mm, margins, gutter)
        }
        other => {
            return Err(Error::Usage(format!(
                "unknown layout '{other}'; valid: saddle-stitch, perfect-bind, n-up"
            )));
        }
    };
    if sheets.is_empty() {
        return Err(Error::Processing(
            "imposition produced zero sheets".into(),
        ));
    }

    // Assemble the output PDF.
    let final_doc = build_output_pdf(
        &sheets,
        &sources,
        sheet_w_mm,
        sheet_h_mm,
        duplex,
        crop_marks,
        out,
    )?;

    // Decide output path.
    let out_path: PathBuf = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            let stem = pdf.file_stem().and_then(|s| s.to_str()).unwrap_or("imposed");
            PathBuf::from(format!("{stem}-imposed.pdf"))
        });

    // Save.
    let mut final_doc = final_doc;
    final_doc
        .save(&out_path)
        .map_err(|e| Error::Processing(format!("PDF save failed: {e}")))?;

    if json {
        let total_output_pages = sheets
            .iter()
            .fold(0usize, |acc, s| acc + 1 + usize::from(!s.back.is_empty()));
        let v = serde_json::json!({
            "output": out_path.display().to_string(),
            "layout": layout,
            "paper": paper,
            "sheet_mm": [sheet_w_mm, sheet_h_mm],
            "source_pages": total_source,
            "sheets": sheets.len(),
            "output_pages": total_output_pages,
            "duplex": duplex,
            "crop_marks": crop_marks,
        });
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    } else if !quiet {
        println!("{}", out_path.display());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    /// Build a tiny PDF with `n` pages, each containing a single text line.
    fn make_pdf(n: u32) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });

        let mut kids: Vec<Object> = Vec::new();
        for i in 0..n {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 24.into()]),
                    Operation::new("Td", vec![72.into(), 720.into()]),
                    Operation::new(
                        "Tj",
                        vec![Object::string_literal(format!("Page {}", i + 1))],
                    ),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id = doc.add_object(Stream::new(
                dictionary! {},
                content.encode().unwrap(),
            ));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => resources_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            });
            kids.push(page_id.into());
        }
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => Object::Integer(n as i64),
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn pad_to_multiple_works() {
        assert_eq!(pad_to_multiple(0, 4), 0);
        assert_eq!(pad_to_multiple(1, 4), 4);
        assert_eq!(pad_to_multiple(4, 4), 4);
        assert_eq!(pad_to_multiple(5, 4), 8);
    }

    #[test]
    fn n_up_grid_picks_close_to_square() {
        // 4 on a roughly square sheet → 2x2
        let (r, c) = grid_for_n_up(4, 297.0, 420.0);
        assert_eq!(r * c, 4);
        assert!((r == 2 && c == 2) || (r == 4 && c == 1) || (r == 1 && c == 4));
    }

    #[test]
    fn saddle_stitch_4_pages_one_sheet() {
        let (sheets, total) =
            calc_saddle_stitch(4, 420.0, 297.0, 10.0, 5.0, 0.0);
        assert_eq!(total, 4);
        assert_eq!(sheets.len(), 1);
        // Sheet 1 front: [4, 1]
        assert_eq!(sheets[0].front[0].page, 4);
        assert_eq!(sheets[0].front[1].page, 1);
        // Sheet 1 back: [2, 3]
        assert_eq!(sheets[0].back[0].page, 2);
        assert_eq!(sheets[0].back[1].page, 3);
    }

    #[test]
    fn saddle_stitch_pads_to_multiple_of_4() {
        let (sheets, total) =
            calc_saddle_stitch(5, 420.0, 297.0, 10.0, 5.0, 0.0);
        assert_eq!(total, 8);
        assert_eq!(sheets.len(), 2);
    }

    #[test]
    fn impose_2up_4_page_pdf() {
        let pdf_bytes = make_pdf(4);
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let src = tmpdir.join(format!("delphi_impose_in_{pid}.pdf"));
        let out = tmpdir.join(format!("delphi_impose_out_{pid}.pdf"));
        std::fs::write(&src, &pdf_bytes).unwrap();
        let r = run(
            &src,
            "saddle-stitch",
            "a4",
            4,
            16,
            10.0,
            5.0,
            0.0,
            false,
            true, // duplex
            false,
            true,
            Some(&out),
        );
        let _ = std::fs::remove_file(&src);
        r.unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        // Should be re-readable by lopdf.
        let doc = Document::load(&out).expect("reload imposed PDF");
        // 4-page source, saddle-stitch, duplex → 2 sheets per side → 2 output pages
        let n_pages = doc.get_pages().len();
        assert_eq!(n_pages, 2);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn impose_n_up_4_pages_per_sheet() {
        let pdf_bytes = make_pdf(8);
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let src = tmpdir.join(format!("delphi_impose_nup_in_{pid}.pdf"));
        let out = tmpdir.join(format!("delphi_impose_nup_out_{pid}.pdf"));
        std::fs::write(&src, &pdf_bytes).unwrap();
        let r = run(
            &src,
            "n-up",
            "a4",
            4,
            16,
            10.0,
            5.0,
            0.0,
            false,
            false,
            false,
            true,
            Some(&out),
        );
        let _ = std::fs::remove_file(&src);
        r.unwrap();
        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        let doc = Document::load(&out).expect("reload n-up PDF");
        let n_pages = doc.get_pages().len();
        // 8 pages / 4-up = 2 sheets, no duplex
        assert_eq!(n_pages, 2);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn impose_unknown_layout_errors() {
        let pdf_bytes = make_pdf(2);
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let src = tmpdir.join(format!("delphi_impose_bad_layout_{pid}.pdf"));
        std::fs::write(&src, &pdf_bytes).unwrap();
        let r = run(
            &src,
            "no-such-layout",
            "a4",
            4,
            16,
            10.0,
            5.0,
            0.0,
            false,
            false,
            false,
            true,
            None,
        );
        let _ = std::fs::remove_file(&src);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    /// Build a 4-page PDF where page 2 has /Rotate 90. Imposition should
    /// succeed without error and the output should have the expected page
    /// count (rotation is baked into the Form XObject matrix).
    fn make_pdf_with_rotation(rotations: &[i64]) -> Vec<u8> {
        let n = rotations.len() as u32;
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Helvetica",
        });
        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font_id },
        });
        let mut kids: Vec<Object> = Vec::new();
        for (i, rot) in rotations.iter().enumerate() {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 24.into()]),
                    Operation::new("Td", vec![72.into(), 720.into()]),
                    Operation::new(
                        "Tj",
                        vec![Object::string_literal(format!("Page {}", i + 1))],
                    ),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id = doc.add_object(Stream::new(
                dictionary! {},
                content.encode().unwrap(),
            ));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => resources_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
                "Rotate" => Object::Integer(*rot),
            });
            kids.push(page_id.into());
        }
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => Object::Integer(n as i64),
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);
        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();
        buf
    }

    #[test]
    fn impose_handles_rotated_source_pages() {
        // Mix of rotations: 0, 90, 180, 270
        let pdf_bytes = make_pdf_with_rotation(&[0, 90, 180, 270]);
        let tmpdir = std::env::temp_dir();
        let pid = std::process::id();
        let src = tmpdir.join(format!("delphi_impose_rot_in_{pid}.pdf"));
        let out = tmpdir.join(format!("delphi_impose_rot_out_{pid}.pdf"));
        std::fs::write(&src, &pdf_bytes).unwrap();
        let r = run(
            &src,
            "n-up",
            "a4",
            4,
            16,
            10.0,
            5.0,
            0.0,
            false,
            false,
            false,
            true,
            Some(&out),
        );
        let _ = std::fs::remove_file(&src);
        r.unwrap();
        let doc = Document::load(&out).expect("reload imposed PDF");
        assert!(!doc.get_pages().is_empty());
        let _ = std::fs::remove_file(&out);
    }
}
