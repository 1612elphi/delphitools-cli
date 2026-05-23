use crate::error::Error;
use serde_json::json;

pub struct PaperSize {
    pub name: &'static str,
    pub series: &'static str,
    /// Width in mm (portrait orientation — shorter edge first)
    pub width_mm: f64,
    /// Height in mm
    pub height_mm: f64,
}

/// Look up a paper size by case-insensitive name, returning (width_mm, height_mm)
/// in portrait orientation.
pub fn lookup_mm(name: &str) -> Result<(f64, f64), Error> {
    SIZES
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .map(|p| (p.width_mm, p.height_mm))
        .ok_or_else(|| Error::Usage(format!("unknown paper size '{name}'")))
}

pub const SIZES: &[PaperSize] = &[
    // ISO A series
    PaperSize { name: "A0",        series: "a", width_mm: 841.0,   height_mm: 1189.0  },
    PaperSize { name: "A1",        series: "a", width_mm: 594.0,   height_mm: 841.0   },
    PaperSize { name: "A2",        series: "a", width_mm: 420.0,   height_mm: 594.0   },
    PaperSize { name: "A3",        series: "a", width_mm: 297.0,   height_mm: 420.0   },
    PaperSize { name: "A4",        series: "a", width_mm: 210.0,   height_mm: 297.0   },
    PaperSize { name: "A5",        series: "a", width_mm: 148.0,   height_mm: 210.0   },
    PaperSize { name: "A6",        series: "a", width_mm: 105.0,   height_mm: 148.0   },
    PaperSize { name: "A7",        series: "a", width_mm: 74.0,    height_mm: 105.0   },
    PaperSize { name: "A8",        series: "a", width_mm: 52.0,    height_mm: 74.0    },
    PaperSize { name: "A9",        series: "a", width_mm: 37.0,    height_mm: 52.0    },
    PaperSize { name: "A10",       series: "a", width_mm: 26.0,    height_mm: 37.0    },
    // ISO B series
    PaperSize { name: "B0",        series: "b", width_mm: 1000.0,  height_mm: 1414.0  },
    PaperSize { name: "B1",        series: "b", width_mm: 707.0,   height_mm: 1000.0  },
    PaperSize { name: "B2",        series: "b", width_mm: 500.0,   height_mm: 707.0   },
    PaperSize { name: "B3",        series: "b", width_mm: 353.0,   height_mm: 500.0   },
    PaperSize { name: "B4",        series: "b", width_mm: 250.0,   height_mm: 353.0   },
    PaperSize { name: "B5",        series: "b", width_mm: 176.0,   height_mm: 250.0   },
    PaperSize { name: "B6",        series: "b", width_mm: 125.0,   height_mm: 176.0   },
    PaperSize { name: "B7",        series: "b", width_mm: 88.0,    height_mm: 125.0   },
    PaperSize { name: "B8",        series: "b", width_mm: 62.0,    height_mm: 88.0    },
    PaperSize { name: "B9",        series: "b", width_mm: 44.0,    height_mm: 62.0    },
    PaperSize { name: "B10",       series: "b", width_mm: 31.0,    height_mm: 44.0    },
    // ISO C series (envelope)
    PaperSize { name: "C0",        series: "c", width_mm: 917.0,   height_mm: 1297.0  },
    PaperSize { name: "C1",        series: "c", width_mm: 648.0,   height_mm: 917.0   },
    PaperSize { name: "C2",        series: "c", width_mm: 458.0,   height_mm: 648.0   },
    PaperSize { name: "C3",        series: "c", width_mm: 324.0,   height_mm: 458.0   },
    PaperSize { name: "C4",        series: "c", width_mm: 229.0,   height_mm: 324.0   },
    PaperSize { name: "C5",        series: "c", width_mm: 162.0,   height_mm: 229.0   },
    PaperSize { name: "C6",        series: "c", width_mm: 114.0,   height_mm: 162.0   },
    PaperSize { name: "C7",        series: "c", width_mm: 81.0,    height_mm: 114.0   },
    PaperSize { name: "C8",        series: "c", width_mm: 57.0,    height_mm: 81.0    },
    PaperSize { name: "C9",        series: "c", width_mm: 40.0,    height_mm: 57.0    },
    PaperSize { name: "C10",       series: "c", width_mm: 28.0,    height_mm: 40.0    },
    // US sizes
    PaperSize { name: "Letter",    series: "us", width_mm: 215.9,  height_mm: 279.4   },
    PaperSize { name: "Legal",     series: "us", width_mm: 215.9,  height_mm: 355.6   },
    PaperSize { name: "Tabloid",   series: "us", width_mm: 279.4,  height_mm: 431.8   },
    PaperSize { name: "Executive", series: "us", width_mm: 184.15, height_mm: 266.7   },
];

const MM_PER_IN: f64 = 25.4;
const PT_PER_IN: f64 = 72.0;

fn mm_to_unit(mm: f64, unit: &str, dpi: f64) -> f64 {
    match unit {
        "in" => mm / MM_PER_IN,
        "pt" => mm / MM_PER_IN * PT_PER_IN,
        "px" => mm / MM_PER_IN * dpi,
        _ => mm, // "mm"
    }
}

fn format_dim(v: f64, unit: &str) -> String {
    match unit {
        "in" => format!("{:.2}", v),
        "pt" => format!("{:.1}", v),
        "px" => format!("{}", v.round() as u64),
        _ => {
            // mm: drop trailing zeros after one decimal
            if v.fract() == 0.0 {
                format!("{}", v as u64)
            } else {
                // Use up to 2 decimal places, strip trailing zeros
                let s = format!("{:.2}", v);
                s.trim_end_matches('0').to_string()
            }
        }
    }
}

fn format_size(size: &PaperSize, unit: &str, dpi: f64, with_px: bool) -> String {
    let w = mm_to_unit(size.width_mm, unit, dpi);
    let h = mm_to_unit(size.height_mm, unit, dpi);
    let base = format!(
        "{}: {} × {} {}",
        size.name,
        format_dim(w, unit),
        format_dim(h, unit),
        unit
    );
    if with_px && unit != "px" {
        let pw = (size.width_mm / MM_PER_IN * dpi).round() as u64;
        let ph = (size.height_mm / MM_PER_IN * dpi).round() as u64;
        format!("{base} ({pw} × {ph} px @{dpi:.0}dpi)")
    } else {
        base
    }
}

pub fn run(
    name: Option<&str>,
    series: Option<&str>,
    unit: &str,
    dpi: f64,
    with_px: bool,
    as_json: bool,
) -> Result<(), Error> {
    // Validate unit
    if !["mm", "in", "pt", "px"].contains(&unit) {
        return Err(Error::Usage(format!(
            "unknown unit '{unit}'; valid units: mm, in, pt, px"
        )));
    }

    // --series: list all sizes in that series
    if let Some(s) = series {
        let s_lower = s.to_ascii_lowercase();
        let matches: Vec<_> = SIZES.iter().filter(|p| p.series == s_lower).collect();
        if matches.is_empty() {
            return Err(Error::Usage(format!(
                "unknown series '{s}'; valid series: a, b, c, us"
            )));
        }
        if as_json {
            let arr: Vec<_> = matches
                .iter()
                .map(|p| size_to_json(p, unit, dpi))
                .collect();
            println!("{}", serde_json::to_string_pretty(&arr).unwrap());
        } else {
            for p in &matches {
                println!("{}", format_size(p, unit, dpi, with_px));
            }
        }
        return Ok(());
    }

    // Positional name argument
    let name = name.ok_or_else(|| {
        Error::Usage("provide a paper size name (e.g. A4, Letter) or --series a|b|c|us".into())
    })?;

    let size = SIZES
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            Error::Usage(format!(
                "unknown paper size '{name}'; try --series a|b|c|us to list sizes"
            ))
        })?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&size_to_json(size, unit, dpi)).unwrap());
    } else {
        println!("{}", format_size(size, unit, dpi, with_px));
    }

    Ok(())
}

fn size_to_json(size: &PaperSize, unit: &str, dpi: f64) -> serde_json::Value {
    let w = mm_to_unit(size.width_mm, unit, dpi);
    let h = mm_to_unit(size.height_mm, unit, dpi);
    json!({
        "name":   size.name,
        "width":  round2(w),
        "height": round2(h),
        "unit":   unit,
    })
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_series_count() {
        let a: Vec<_> = SIZES.iter().filter(|p| p.series == "a").collect();
        assert_eq!(a.len(), 11); // A0–A10
    }

    #[test]
    fn b_series_count() {
        let b: Vec<_> = SIZES.iter().filter(|p| p.series == "b").collect();
        assert_eq!(b.len(), 11); // B0–B10
    }

    #[test]
    fn c_series_count() {
        let c: Vec<_> = SIZES.iter().filter(|p| p.series == "c").collect();
        assert_eq!(c.len(), 11); // C0–C10
    }

    #[test]
    fn us_series_count() {
        let us: Vec<_> = SIZES.iter().filter(|p| p.series == "us").collect();
        assert_eq!(us.len(), 4);
    }

    #[test]
    fn a4_dimensions() {
        let a4 = SIZES.iter().find(|p| p.name == "A4").unwrap();
        assert_eq!(a4.width_mm, 210.0);
        assert_eq!(a4.height_mm, 297.0);
    }

    #[test]
    fn letter_dimensions() {
        let letter = SIZES.iter().find(|p| p.name == "Letter").unwrap();
        assert!((letter.width_mm - 215.9).abs() < 1e-9);
        assert!((letter.height_mm - 279.4).abs() < 1e-9);
    }

    #[test]
    fn mm_to_in_conversion() {
        let v = mm_to_unit(210.0, "in", 72.0);
        assert!((v - 210.0 / 25.4).abs() < 1e-6);
    }

    #[test]
    fn mm_to_px_at_300dpi() {
        let w = mm_to_unit(210.0, "px", 300.0).round() as u64;
        let h = mm_to_unit(297.0, "px", 300.0).round() as u64;
        assert_eq!(w, 2480);
        assert_eq!(h, 3508);
    }

    #[test]
    fn run_a4_default() {
        run(Some("a4"), None, "mm", 72.0, false, false).unwrap();
    }

    #[test]
    fn run_a4_with_dpi() {
        run(Some("A4"), None, "mm", 300.0, true, false).unwrap();
    }

    #[test]
    fn run_a4_inches() {
        run(Some("A4"), None, "in", 72.0, false, false).unwrap();
    }

    #[test]
    fn run_a4_json() {
        run(Some("A4"), None, "mm", 72.0, false, true).unwrap();
    }

    #[test]
    fn run_series_a() {
        run(None, Some("a"), "mm", 72.0, false, false).unwrap();
    }

    #[test]
    fn run_series_us_json() {
        run(None, Some("us"), "mm", 72.0, false, true).unwrap();
    }

    #[test]
    fn run_unknown_size_errors() {
        let r = run(Some("Z99"), None, "mm", 72.0, false, false);
        assert!(matches!(r, Err(crate::error::Error::Usage(_))));
    }

    #[test]
    fn run_unknown_series_errors() {
        let r = run(None, Some("z"), "mm", 72.0, false, false);
        assert!(matches!(r, Err(crate::error::Error::Usage(_))));
    }

    #[test]
    fn run_unknown_unit_errors() {
        let r = run(Some("A4"), None, "em", 72.0, false, false);
        assert!(matches!(r, Err(crate::error::Error::Usage(_))));
    }

    #[test]
    fn run_no_args_errors() {
        let r = run(None, None, "mm", 72.0, false, false);
        assert!(matches!(r, Err(crate::error::Error::Usage(_))));
    }

    #[test]
    fn format_dim_mm_integer() {
        assert_eq!(format_dim(210.0, "mm"), "210");
    }

    #[test]
    fn format_dim_mm_fractional() {
        assert_eq!(format_dim(215.9, "mm"), "215.9");
    }

    #[test]
    fn format_dim_in() {
        // 210mm / 25.4 = 8.267...
        let v = mm_to_unit(210.0, "in", 72.0);
        let s = format_dim(v, "in");
        assert_eq!(s, "8.27");
    }
}
