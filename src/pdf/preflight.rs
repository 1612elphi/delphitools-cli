//! PDF preflight: structural and content analysis for print-readiness.
//!
//! Uses lopdf to walk the page tree and resource dictionaries, reporting:
//!   - PDF version, page count, encrypted status
//!   - Per-page: MediaBox, TrimBox, BleedBox dimensions; page rotation
//!   - Fonts: name, subtype, embedded status
//!   - Images: dimensions and a conservative full-page DPI estimate
//!   - Transparency: page-level Group/S=Transparency and ExtGState ca/CA/SMask
//!   - Spot colours: DeviceN and Separation colorspaces in resource dicts

use crate::error::Error;
use lopdf::{Document, Object, ObjectId};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const PT_PER_MM: f64 = 72.0 / 25.4;

/// 14 standard Type 1 fonts that may legally be unembedded in a PDF.
const STANDARD_14_FONTS: &[&str] = &[
    "Courier",
    "Courier-Bold",
    "Courier-BoldOblique",
    "Courier-Oblique",
    "Helvetica",
    "Helvetica-Bold",
    "Helvetica-BoldOblique",
    "Helvetica-Oblique",
    "Symbol",
    "Times-Bold",
    "Times-BoldItalic",
    "Times-Italic",
    "Times-Roman",
    "ZapfDingbats",
];

#[derive(Debug, Clone, Copy)]
struct Box4 {
    /// Width in points
    w: f64,
    /// Height in points
    h: f64,
}

fn pt_to_mm(pt: f64) -> f64 {
    pt / PT_PER_MM
}

fn read_box(obj: &Object) -> Option<Box4> {
    let arr = obj.as_array().ok()?;
    if arr.len() < 4 {
        return None;
    }
    let nums: Vec<f64> = arr
        .iter()
        .filter_map(|o| match o {
            Object::Integer(i) => Some(*i as f64),
            Object::Real(r) => Some(*r as f64),
            _ => None,
        })
        .collect();
    if nums.len() < 4 {
        return None;
    }
    let (llx, lly, urx, ury) = (nums[0], nums[1], nums[2], nums[3]);
    Some(Box4 {
        w: (urx - llx).abs(),
        h: (ury - lly).abs(),
    })
}

#[derive(Debug, Default)]
struct PageReport {
    page_num: u32,
    media_box: Option<Box4>,
    trim_box: Option<Box4>,
    bleed_box: Option<Box4>,
    rotation: i64,
    image_count: usize,
    image_dpis: Vec<u32>, // conservative per-page-fill DPI estimates
    has_transparency: bool,
    spot_colour_spaces: Vec<String>,
}

#[derive(Debug, Default)]
struct FontReport {
    name: String,
    subtype: String,
    embedded: bool,
}

#[derive(Debug)]
struct Report {
    pdf_version: String,
    page_count: usize,
    encrypted: bool,
    pages: Vec<PageReport>,
    fonts: Vec<FontReport>,
    warnings: Vec<String>,
}

/// Resolve a reference if needed, returning the inner dict if any.
fn resolve_dict<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a lopdf::Dictionary> {
    match obj {
        Object::Dictionary(d) => Some(d),
        Object::Reference(id) => doc.get_dictionary(*id).ok(),
        _ => None,
    }
}

/// Resolve a value through one level of Reference if needed.
fn deref<'a>(doc: &'a Document, obj: &'a Object) -> &'a Object {
    if let Object::Reference(id) = obj {
        if let Ok(o) = doc.get_object(*id) {
            return o;
        }
    }
    obj
}

/// Get a value from a dict, dereferencing if it's a Reference.
fn dict_get<'a>(doc: &'a Document, dict: &'a lopdf::Dictionary, key: &[u8]) -> Option<&'a Object> {
    let v = dict.get(key).ok()?;
    Some(deref(doc, v))
}

/// Collect page-level info, including walking Parent chain for inherited MediaBox/Rotate.
fn inherited_get<'a>(doc: &'a Document, page_id: ObjectId, key: &[u8]) -> Option<Object> {
    let mut id = page_id;
    for _ in 0..16 {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(v) = dict.get(key) {
            // Deref one level then clone
            return Some(deref(doc, v).clone());
        }
        match dict.get(b"Parent").ok() {
            Some(Object::Reference(pid)) => id = *pid,
            _ => return None,
        }
    }
    None
}

/// Walk the Font subdict of a page's resources, accumulating font info into `fonts`.
fn scan_fonts_in_resources(
    doc: &Document,
    resources: &lopdf::Dictionary,
    fonts: &mut BTreeMap<String, FontReport>,
) {
    let Some(fonts_obj) = dict_get(doc, resources, b"Font") else {
        return;
    };
    let Some(fonts_dict) = resolve_dict(doc, fonts_obj) else {
        return;
    };
    for (_alias, font_value) in fonts_dict.iter() {
        let Some(font_dict) = resolve_dict(doc, font_value) else {
            continue;
        };
        let name = font_dict
            .get(b"BaseFont")
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        // Strip subset prefix "ABCDEF+"
        let clean = if name.len() > 7 && name.as_bytes()[6] == b'+' {
            name[7..].to_string()
        } else {
            name.clone()
        };
        let subtype = font_dict
            .get(b"Subtype")
            .ok()
            .and_then(|o| o.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).to_string())
            .unwrap_or_default();

        // Embedded check: FontDescriptor has FontFile / FontFile2 / FontFile3, or
        // for Type0 the descendant CIDFont has one.
        let mut embedded = false;
        if let Some(desc_obj) = dict_get(doc, font_dict, b"FontDescriptor") {
            if let Some(desc) = resolve_dict(doc, desc_obj) {
                if desc.get(b"FontFile").is_ok()
                    || desc.get(b"FontFile2").is_ok()
                    || desc.get(b"FontFile3").is_ok()
                {
                    embedded = true;
                }
            }
        }
        if !embedded && subtype == "Type0" {
            if let Some(desc_obj) = dict_get(doc, font_dict, b"DescendantFonts") {
                if let Object::Array(arr) = deref(doc, desc_obj) {
                    for el in arr {
                        if let Some(cid_dict) = resolve_dict(doc, el) {
                            if let Some(cdesc_obj) = dict_get(doc, cid_dict, b"FontDescriptor") {
                                if let Some(cdesc) = resolve_dict(doc, cdesc_obj) {
                                    if cdesc.get(b"FontFile").is_ok()
                                        || cdesc.get(b"FontFile2").is_ok()
                                        || cdesc.get(b"FontFile3").is_ok()
                                    {
                                        embedded = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Standard 14 fonts are effectively embedded (printer supplies them).
        if !embedded && STANDARD_14_FONTS.iter().any(|s| *s == clean) {
            embedded = true;
        }

        let key = format!("{clean}\u{0}{subtype}");
        fonts.entry(key).or_insert(FontReport {
            name: clean,
            subtype,
            embedded,
        });
    }
}

/// Inspect ExtGState dict for transparency markers (ca/CA<1 or SMask not None).
fn extgstate_has_transparency(doc: &Document, ext_gs: &lopdf::Dictionary) -> bool {
    for (_alias, gs_value) in ext_gs.iter() {
        let Some(gs) = resolve_dict(doc, gs_value) else {
            continue;
        };
        if let Ok(ca) = gs.get(b"ca") {
            if let Ok(v) = ca.as_float() {
                if v < 1.0 {
                    return true;
                }
            } else if let Ok(i) = ca.as_i64() {
                if i < 1 {
                    return true;
                }
            }
        }
        if let Ok(ca) = gs.get(b"CA") {
            if let Ok(v) = ca.as_float() {
                if v < 1.0 {
                    return true;
                }
            } else if let Ok(i) = ca.as_i64() {
                if i < 1 {
                    return true;
                }
            }
        }
        if let Ok(smask) = gs.get(b"SMask") {
            // SMask is either /None or a stream — anything not the name "None" is real
            let smask = deref(doc, smask);
            match smask {
                Object::Name(n) if n == b"None" => {}
                _ => return true,
            }
        }
    }
    false
}

/// Scan the ColorSpace dict in resources for Separation / DeviceN spaces (spot colours).
fn collect_spot_colourspaces(doc: &Document, resources: &lopdf::Dictionary, out: &mut Vec<String>) {
    let Some(cs_obj) = dict_get(doc, resources, b"ColorSpace") else {
        return;
    };
    let Some(cs_dict) = resolve_dict(doc, cs_obj) else {
        return;
    };
    for (alias, cs_value) in cs_dict.iter() {
        let cs = deref(doc, cs_value);
        let head = match cs {
            Object::Array(arr) => arr
                .first()
                .and_then(|o| o.as_name().ok())
                .map(|n| String::from_utf8_lossy(n).to_string()),
            Object::Name(n) => Some(String::from_utf8_lossy(n).to_string()),
            _ => None,
        };
        if let Some(head) = head {
            if head == "Separation" || head == "DeviceN" {
                let alias = String::from_utf8_lossy(alias).to_string();
                out.push(format!("{alias} ({head})"));
            }
        }
    }
}

fn analyse(path: &Path) -> Result<Report, Error> {
    let mut doc = Document::load(path).map_err(|e| Error::Input(format!("PDF load failed: {e}")))?;
    let pdf_version = doc.version.clone();
    let encrypted = doc.is_encrypted();

    // Permit reading the structure even if encrypted — try empty password.
    if encrypted {
        let _ = doc.decrypt("");
    }

    let pages: BTreeMap<u32, ObjectId> = doc.get_pages();
    let mut page_reports: Vec<PageReport> = Vec::with_capacity(pages.len());
    let mut fonts_map: BTreeMap<String, FontReport> = BTreeMap::new();

    for (page_num, page_id) in &pages {
        let mut pr = PageReport {
            page_num: *page_num,
            ..Default::default()
        };

        // MediaBox / TrimBox / BleedBox (with parent inheritance for MediaBox)
        if let Some(mb) = inherited_get(&doc, *page_id, b"MediaBox") {
            pr.media_box = read_box(&mb);
        }
        if let Ok(page_dict) = doc.get_dictionary(*page_id) {
            if let Ok(v) = page_dict.get(b"TrimBox") {
                pr.trim_box = read_box(deref(&doc, v));
            }
            if let Ok(v) = page_dict.get(b"BleedBox") {
                pr.bleed_box = read_box(deref(&doc, v));
            }
        }
        if let Some(rot) = inherited_get(&doc, *page_id, b"Rotate") {
            pr.rotation = rot.as_i64().unwrap_or(0);
        }

        // Resources: fonts, images, transparency, spot colours
        if let Ok(page_dict) = doc.get_dictionary(*page_id) {
            // Page-level Group (transparency)
            if let Ok(group_obj) = page_dict.get(b"Group") {
                if let Some(group) = resolve_dict(&doc, group_obj) {
                    if let Ok(Object::Name(n)) = group.get(b"S") {
                        if n == b"Transparency" {
                            pr.has_transparency = true;
                        }
                    }
                }
            }
        }

        // Walk resources (may be on the page or inherited from a Pages parent)
        let (resource_dict, resource_ids) = doc
            .get_page_resources(*page_id)
            .unwrap_or((None, Vec::new()));

        // Collect every Resources dict we should consider for this page
        let mut resource_dicts: Vec<&lopdf::Dictionary> = Vec::new();
        if let Some(d) = resource_dict {
            resource_dicts.push(d);
        }
        for rid in &resource_ids {
            if let Ok(d) = doc.get_dictionary(*rid) {
                resource_dicts.push(d);
            }
        }

        for res in &resource_dicts {
            scan_fonts_in_resources(&doc, res, &mut fonts_map);
            // ExtGState transparency
            if let Some(ext_gs_obj) = dict_get(&doc, res, b"ExtGState") {
                if let Some(ext_gs) = resolve_dict(&doc, ext_gs_obj) {
                    if extgstate_has_transparency(&doc, ext_gs) {
                        pr.has_transparency = true;
                    }
                }
            }
            // Spot colours
            collect_spot_colourspaces(&doc, res, &mut pr.spot_colour_spaces);
        }

        // Images via lopdf's helper
        if let Ok(imgs) = doc.get_page_images(*page_id) {
            pr.image_count = imgs.len();
            if let Some(mb) = pr.media_box {
                let w_in = mb.w / 72.0;
                let h_in = mb.h / 72.0;
                if w_in > 0.0 && h_in > 0.0 {
                    for img in &imgs {
                        let dpi_x = img.width as f64 / w_in;
                        let dpi_y = img.height as f64 / h_in;
                        let dpi = dpi_x.min(dpi_y).round() as u32;
                        pr.image_dpis.push(dpi);
                    }
                }
            }
        }

        page_reports.push(pr);
    }

    let fonts: Vec<FontReport> = fonts_map.into_values().collect();

    // Compose warnings
    let mut warnings: Vec<String> = Vec::new();
    if encrypted {
        warnings.push("PDF is encrypted or carries security restrictions".to_string());
    }
    for f in &fonts {
        if !f.embedded {
            warnings.push(format!("Font \"{}\" is not embedded", f.name));
        }
    }
    for p in &page_reports {
        if p.bleed_box.is_none() {
            warnings.push(format!("Page {}: no BleedBox defined", p.page_num));
        }
        for dpi in &p.image_dpis {
            if *dpi < 150 {
                warnings.push(format!(
                    "Page {}: low-resolution image ({} DPI at full page)",
                    p.page_num, dpi
                ));
            }
        }
    }
    // PDF version warning
    if let Ok(version_num) = pdf_version.parse::<f32>() {
        if version_num < 1.4 {
            warnings.push(format!(
                "PDF version {pdf_version} is below 1.4 (no transparency support)"
            ));
        }
    }
    // Mixed-orientation warning
    if page_reports.len() > 1 {
        let mut orientations: BTreeSet<&'static str> = BTreeSet::new();
        for p in &page_reports {
            if let Some(mb) = p.media_box {
                orientations.insert(if mb.w > mb.h { "landscape" } else { "portrait" });
            }
        }
        if orientations.len() > 1 {
            warnings.push("Mixed page orientations detected".to_string());
        }
    }

    Ok(Report {
        pdf_version,
        page_count: page_reports.len(),
        encrypted,
        pages: page_reports,
        fonts,
        warnings,
    })
}

fn print_human(report: &Report, path: &Path) {
    println!("Preflight: {}", path.display());
    println!("PDF version: {}", report.pdf_version);
    println!("Pages: {}", report.page_count);
    println!("Encrypted: {}", if report.encrypted { "yes" } else { "no" });

    println!("\nPages:");
    for p in &report.pages {
        let mb = p
            .media_box
            .map(|b| format!("{:.1} × {:.1} mm", pt_to_mm(b.w), pt_to_mm(b.h)))
            .unwrap_or_else(|| "—".to_string());
        let trim = p
            .trim_box
            .map(|b| format!("trim {:.1}×{:.1} mm", pt_to_mm(b.w), pt_to_mm(b.h)))
            .unwrap_or_else(|| "no TrimBox".to_string());
        let bleed = p
            .bleed_box
            .map(|b| format!("bleed {:.1}×{:.1} mm", pt_to_mm(b.w), pt_to_mm(b.h)))
            .unwrap_or_else(|| "no BleedBox".to_string());
        let rot = if p.rotation != 0 {
            format!(", rotate {}°", p.rotation)
        } else {
            String::new()
        };
        println!("  Page {}: {mb} ({trim}, {bleed}){rot}", p.page_num);
        if p.image_count > 0 {
            let dpi_str = if p.image_dpis.is_empty() {
                String::new()
            } else {
                let min = p.image_dpis.iter().min().copied().unwrap_or(0);
                let max = p.image_dpis.iter().max().copied().unwrap_or(0);
                if min == max {
                    format!(", ~{min} DPI")
                } else {
                    format!(", {min}–{max} DPI")
                }
            };
            println!("    images: {}{dpi_str}", p.image_count);
        }
        if p.has_transparency {
            println!("    transparency: yes");
        }
        if !p.spot_colour_spaces.is_empty() {
            println!("    spot colours: {}", p.spot_colour_spaces.join(", "));
        }
    }

    println!("\nFonts:");
    if report.fonts.is_empty() {
        println!("  (none)");
    } else {
        for f in &report.fonts {
            let mark = if f.embedded { "" } else { "  NOT EMBEDDED" };
            let subtype = if f.subtype.is_empty() {
                String::new()
            } else {
                format!(" [{}]", f.subtype)
            };
            println!("  {}{subtype}{mark}", f.name);
        }
    }

    if !report.warnings.is_empty() {
        println!("\nWarnings ({}):", report.warnings.len());
        for w in &report.warnings {
            println!("  - {w}");
        }
    } else {
        println!("\nNo warnings — looks print-ready.");
    }
}

fn print_json(report: &Report, path: &Path) {
    let pages: Vec<serde_json::Value> = report
        .pages
        .iter()
        .map(|p| {
            let mb = p.media_box.map(|b| {
                json!({
                    "width_mm":  round1(pt_to_mm(b.w)),
                    "height_mm": round1(pt_to_mm(b.h)),
                    "width_pt":  round1(b.w),
                    "height_pt": round1(b.h),
                })
            });
            let trim = p.trim_box.map(|b| {
                json!({
                    "width_mm":  round1(pt_to_mm(b.w)),
                    "height_mm": round1(pt_to_mm(b.h)),
                })
            });
            let bleed = p.bleed_box.map(|b| {
                json!({
                    "width_mm":  round1(pt_to_mm(b.w)),
                    "height_mm": round1(pt_to_mm(b.h)),
                })
            });
            json!({
                "page": p.page_num,
                "media_box": mb,
                "trim_box": trim,
                "bleed_box": bleed,
                "rotation": p.rotation,
                "image_count": p.image_count,
                "image_dpis": p.image_dpis,
                "transparency": p.has_transparency,
                "spot_colour_spaces": p.spot_colour_spaces,
            })
        })
        .collect();

    let fonts: Vec<serde_json::Value> = report
        .fonts
        .iter()
        .map(|f| {
            json!({
                "name": f.name,
                "subtype": f.subtype,
                "embedded": f.embedded,
            })
        })
        .collect();

    let v = json!({
        "file": path.display().to_string(),
        "pdf_version": report.pdf_version,
        "page_count": report.page_count,
        "encrypted": report.encrypted,
        "pages": pages,
        "fonts": fonts,
        "warnings": report.warnings,
    });
    println!("{}", serde_json::to_string_pretty(&v).unwrap());
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

pub fn run(pdf: &Path, json_out: bool) -> Result<(), Error> {
    let report = analyse(pdf)?;
    if json_out {
        print_json(&report, pdf);
    } else {
        print_human(&report, pdf);
    }
    if !report.warnings.is_empty() {
        return Err(Error::Processing(format!(
            "{} warning(s) — see report",
            report.warnings.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    fn make_tiny_pdf() -> Vec<u8> {
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
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Td", vec![50.into(), 700.into()]),
                Operation::new("Tj", vec![Object::string_literal("Hello world")]),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id =
            doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
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
    fn analyse_tiny_pdf() {
        let pdf_bytes = make_tiny_pdf();
        let pid = std::process::id();
        let tmp = std::env::temp_dir().join(format!("delphi_preflight_test_{pid}.pdf"));
        std::fs::write(&tmp, &pdf_bytes).unwrap();
        let report = analyse(&tmp).unwrap();
        assert_eq!(report.page_count, 1);
        assert_eq!(report.pdf_version, "1.5");
        assert!(!report.encrypted);
        // Helvetica is a standard-14 font so it should be treated as embedded.
        let hel = report.fonts.iter().find(|f| f.name == "Helvetica").unwrap();
        assert!(hel.embedded);
        // MediaBox in points: 595x842 -> mm
        let mb = report.pages[0].media_box.unwrap();
        assert!((mb.w - 595.0).abs() < 0.001);
        assert!((mb.h - 842.0).abs() < 0.001);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn run_tiny_pdf_no_warnings_smoketest() {
        let pdf_bytes = make_tiny_pdf();
        let pid = std::process::id();
        let tmp = std::env::temp_dir().join(format!("delphi_preflight_run_{pid}.pdf"));
        std::fs::write(&tmp, &pdf_bytes).unwrap();
        // It WILL warn (no BleedBox), so exit code is non-zero (warnings present).
        let result = run(&tmp, false);
        assert!(result.is_err());
        if let Err(Error::Processing(_)) = result {
            // expected
        } else {
            panic!("expected Processing error for warnings, got: {result:?}");
        }
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn analyse_missing_file_errors() {
        let r = analyse(Path::new("/nonexistent/__delphi_no_such_pdf__.pdf"));
        assert!(matches!(r, Err(Error::Input(_))));
    }
}
