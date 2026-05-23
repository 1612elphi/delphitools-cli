use crate::error::Error;
use serde_json::{Map, Value};

/// 96 px per inch (CSS reference pixel).
const PX_PER_IN: f64 = 96.0;
/// 72 pt per inch (PostScript point).
const PT_PER_IN: f64 = 72.0;
/// 25.4 mm per inch.
const MM_PER_IN: f64 = 25.4;
/// 12 pt per pica.
const PT_PER_PC: f64 = 12.0;

/// Recognised units, in display order.
const UNITS: &[&str] = &["pt", "px", "pc", "in", "mm", "cm", "em", "rem"];

/// Parse an input like `"12pt"`, `"1.5rem"`, `"4.233mm"`, `"12em"` into (value, unit).
///
/// Scientific notation (`1e3px`) is supported: an `e`/`E` is treated as
/// part of the number only when followed by a digit or `+`/`-` then a digit.
/// Otherwise it starts the unit string (so `12em` parses as 12 + "em").
fn parse_value(input: &str) -> Result<(f64, String), Error> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(Error::Usage("typo: empty value".into()));
    }

    let bytes = trimmed.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;

    // Optional sign.
    if i < n && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    // Integer part.
    let int_start = i;
    while i < n && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let has_int = i > int_start;

    // Optional fraction.
    let mut has_frac = false;
    if i < n && bytes[i] == b'.' {
        i += 1;
        let frac_start = i;
        while i < n && bytes[i].is_ascii_digit() {
            i += 1;
        }
        has_frac = i > frac_start;
    }

    if !has_int && !has_frac {
        return Err(Error::Usage(format!("typo: missing number in '{input}'")));
    }

    // Optional exponent — only if `e`/`E` is followed by digit or sign+digit.
    if i < n && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < n && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        if j < n && bytes[j].is_ascii_digit() {
            // It's a real exponent — consume it.
            i = j;
            while i < n && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        // else: leave `e` to start the unit (e.g. "12em", "1.5em").
    }

    let (num_str, unit_str) = trimmed.split_at(i);
    let unit = unit_str.trim().to_ascii_lowercase();

    let value: f64 = num_str.parse().map_err(|_| {
        Error::Usage(format!("typo: cannot parse number '{num_str}' in '{input}'"))
    })?;

    if unit.is_empty() {
        return Err(Error::Usage(format!(
            "typo: missing unit in '{input}' (expected one of: {})",
            UNITS.join(", ")
        )));
    }

    if !UNITS.contains(&unit.as_str()) {
        return Err(Error::Usage(format!(
            "typo: unknown unit '{unit}' (expected one of: {})",
            UNITS.join(", ")
        )));
    }

    Ok((value, unit))
}

/// Convert a value in `unit` to px, given the base font size in px (used for em/rem).
fn to_px(value: f64, unit: &str, base: f64) -> f64 {
    match unit {
        "px" => value,
        "pt" => value * (PX_PER_IN / PT_PER_IN),
        "pc" => value * PT_PER_PC * (PX_PER_IN / PT_PER_IN),
        "in" => value * PX_PER_IN,
        "mm" => value * (PX_PER_IN / MM_PER_IN),
        "cm" => value * (PX_PER_IN / (MM_PER_IN / 10.0)),
        "em" | "rem" => value * base,
        _ => f64::NAN,
    }
}

/// Convert a px value to `unit`, given the base font size in px.
fn from_px(px: f64, unit: &str, base: f64) -> f64 {
    match unit {
        "px" => px,
        "pt" => px * (PT_PER_IN / PX_PER_IN),
        "pc" => px * (PT_PER_IN / PX_PER_IN) / PT_PER_PC,
        "in" => px / PX_PER_IN,
        "mm" => px * (MM_PER_IN / PX_PER_IN),
        "cm" => px * ((MM_PER_IN / 10.0) / PX_PER_IN),
        "em" | "rem" => px / base,
        _ => f64::NAN,
    }
}

/// Trim trailing zeros from a fixed-precision float string,
/// then a dangling decimal point if any.
fn trim_float(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Format a converted value for display: up to 6 decimals, no trailing zeros.
fn format_value(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    // Render with enough precision then trim.
    let raw = format!("{:.6}", v);
    trim_float(&raw)
}

pub fn run(value: &str, targets: &[String], base: f64, as_json: bool) -> Result<(), Error> {
    if !base.is_finite() || base <= 0.0 {
        return Err(Error::Usage(format!(
            "typo: --base must be a positive number, got {base}"
        )));
    }

    let (input_value, input_unit) = parse_value(value)?;
    let px = to_px(input_value, &input_unit, base);

    // Parse targets: may come as ["px,mm,pc"] or ["px","mm","pc"]. Default: all units.
    let target_list: Vec<String> = if targets.is_empty() {
        UNITS.iter().map(|u| u.to_string()).collect()
    } else {
        let mut list = Vec::new();
        for t in targets {
            for piece in t.split(',') {
                let p = piece.trim().to_ascii_lowercase();
                if p.is_empty() {
                    continue;
                }
                if !UNITS.contains(&p.as_str()) {
                    return Err(Error::Usage(format!(
                        "typo: unknown target unit '{p}' (expected one of: {})",
                        UNITS.join(", ")
                    )));
                }
                list.push(p);
            }
        }
        if list.is_empty() {
            UNITS.iter().map(|u| u.to_string()).collect()
        } else {
            list
        }
    };

    if as_json {
        let mut map = Map::new();
        for unit in &target_list {
            let v = from_px(px, unit, base);
            // Round to 6 decimal places to avoid floating-point noise.
            let rounded = (v * 1_000_000.0).round() / 1_000_000.0;
            map.insert(unit.clone(), Value::from(rounded));
        }
        println!("{}", serde_json::to_string_pretty(&Value::Object(map)).unwrap());
        return Ok(());
    }

    if target_list.len() == 1 {
        let unit = &target_list[0];
        let v = from_px(px, unit, base);
        println!("{}{}", format_value(v), unit);
    } else {
        for unit in &target_list {
            let v = from_px(px, unit, base);
            println!("{}: {}{}", unit, format_value(v), unit);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pt() {
        let (v, u) = parse_value("12pt").unwrap();
        assert_eq!(v, 12.0);
        assert_eq!(u, "pt");
    }

    #[test]
    fn parse_decimal_rem() {
        let (v, u) = parse_value("1.5rem").unwrap();
        assert_eq!(v, 1.5);
        assert_eq!(u, "rem");
    }

    #[test]
    fn parse_em() {
        let (v, u) = parse_value("12em").unwrap();
        assert_eq!(v, 12.0);
        assert_eq!(u, "em");
    }

    #[test]
    fn parse_decimal_em() {
        let (v, u) = parse_value("0.5em").unwrap();
        assert_eq!(v, 0.5);
        assert_eq!(u, "em");
    }

    #[test]
    fn parse_scientific_notation() {
        let (v, u) = parse_value("1e3px").unwrap();
        assert_eq!(v, 1000.0);
        assert_eq!(u, "px");
    }

    #[test]
    fn parse_negative_exponent() {
        let (v, u) = parse_value("1.5e-2in").unwrap();
        assert!((v - 0.015).abs() < 1e-12);
        assert_eq!(u, "in");
    }

    #[test]
    fn parse_negative() {
        let (v, u) = parse_value("-4.233mm").unwrap();
        assert!((v - (-4.233)).abs() < 1e-12);
        assert_eq!(u, "mm");
    }

    #[test]
    fn parse_with_space() {
        let (v, u) = parse_value("12 pt").unwrap();
        assert_eq!(v, 12.0);
        assert_eq!(u, "pt");
    }

    #[test]
    fn parse_uppercase_unit() {
        let (_, u) = parse_value("16PX").unwrap();
        assert_eq!(u, "px");
    }

    #[test]
    fn parse_missing_unit() {
        let r = parse_value("12");
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn parse_unknown_unit() {
        let r = parse_value("12xy");
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn pt_to_px_at_72_eq_96() {
        let v = from_px(to_px(72.0, "pt", 16.0), "px", 16.0);
        assert!((v - 96.0).abs() < 1e-9);
    }

    #[test]
    fn one_pica_eq_12pt() {
        let v = from_px(to_px(1.0, "pc", 16.0), "pt", 16.0);
        assert!((v - 12.0).abs() < 1e-9);
    }

    #[test]
    fn one_inch_eq_25_4mm() {
        let v = from_px(to_px(1.0, "in", 16.0), "mm", 16.0);
        assert!((v - 25.4).abs() < 1e-9);
    }

    #[test]
    fn one_inch_eq_72pt() {
        let v = from_px(to_px(1.0, "in", 16.0), "pt", 16.0);
        assert!((v - 72.0).abs() < 1e-9);
    }

    #[test]
    fn one_inch_eq_96px() {
        let v = to_px(1.0, "in", 16.0);
        assert!((v - 96.0).abs() < 1e-9);
    }

    #[test]
    fn one_rem_eq_base_px() {
        let v = from_px(to_px(1.0, "rem", 16.0), "px", 16.0);
        assert_eq!(v, 16.0);
        let v = from_px(to_px(1.0, "rem", 20.0), "px", 20.0);
        assert_eq!(v, 20.0);
    }

    #[test]
    fn em_eq_rem() {
        let a = to_px(2.0, "em", 16.0);
        let b = to_px(2.0, "rem", 16.0);
        assert_eq!(a, b);
    }

    #[test]
    fn cm_eq_10_mm() {
        let a = to_px(1.0, "cm", 16.0);
        let b = to_px(10.0, "mm", 16.0);
        assert!((a - b).abs() < 1e-9);
    }

    #[test]
    fn round_trip_pt() {
        for unit in ["pt", "px", "pc", "in", "mm", "cm", "em", "rem"] {
            let original = 12.5;
            let px = to_px(original, unit, 16.0);
            let back = from_px(px, unit, 16.0);
            assert!((back - original).abs() < 1e-9, "round trip failed for {unit}");
        }
    }

    #[test]
    fn format_trims_trailing_zeros() {
        assert_eq!(format_value(16.0), "16");
        assert_eq!(format_value(1.5), "1.5");
        assert_eq!(format_value(0.0), "0");
    }

    #[test]
    fn run_basic() {
        run("12pt", &[], 16.0, false).unwrap();
    }

    #[test]
    fn run_single_target() {
        run("12pt", &["px".into()], 16.0, false).unwrap();
    }

    #[test]
    fn run_csv_targets() {
        run("12pt", &["px,mm,pc".into()], 16.0, false).unwrap();
    }

    #[test]
    fn run_json() {
        run("12pt", &[], 16.0, true).unwrap();
    }

    #[test]
    fn run_unknown_unit_errors() {
        let r = run("12xy", &[], 16.0, false);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn run_bad_base() {
        let r = run("12pt", &[], 0.0, false);
        assert!(matches!(r, Err(Error::Usage(_))));
    }
}
