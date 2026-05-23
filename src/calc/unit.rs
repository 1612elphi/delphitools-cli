use crate::error::Error;
use serde_json::json;

// ---------------------------------------------------------------------------
// Unit table
// ---------------------------------------------------------------------------

/// A unit's category. Two units can only be converted to each other if they
/// share a category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Length,
    Mass,
    Volume,
    Time,
    Area,
    Speed,
    Pressure,
    Energy,
    Data,
    Temperature,
}

impl Category {
    fn name(self) -> &'static str {
        match self {
            Category::Length => "length",
            Category::Mass => "mass",
            Category::Volume => "volume",
            Category::Time => "time",
            Category::Area => "area",
            Category::Speed => "speed",
            Category::Pressure => "pressure",
            Category::Energy => "energy",
            Category::Data => "data",
            Category::Temperature => "temperature",
        }
    }
}

/// A unit. For linear units, we keep `factor`: value-in-base = `factor * value`.
/// For temperature, we keep formula closures.
struct UnitDef {
    name: &'static str,
    category: Category,
    /// linear factor to base unit; ignored for temperature
    factor: f64,
}

/// All known units, keyed by their canonical lookup string.
/// We match case-sensitively first, then fall back to a lowercase lookup.
fn units() -> &'static [(&'static str, UnitDef)] {
    // Bases per category:
    //   length: m, mass: kg, volume: l, time: s, area: m2, speed: m/s,
    //   pressure: pa, energy: j, data: byte (B), temperature: c
    &[
        // ── length (base: m) ────────────────────────────────────────────────
        ("mm",  UnitDef { name: "mm",  category: Category::Length, factor: 0.001 }),
        ("cm",  UnitDef { name: "cm",  category: Category::Length, factor: 0.01 }),
        ("m",   UnitDef { name: "m",   category: Category::Length, factor: 1.0 }),
        ("km",  UnitDef { name: "km",  category: Category::Length, factor: 1000.0 }),
        ("in",  UnitDef { name: "in",  category: Category::Length, factor: 0.0254 }),
        ("ft",  UnitDef { name: "ft",  category: Category::Length, factor: 0.3048 }),
        ("yd",  UnitDef { name: "yd",  category: Category::Length, factor: 0.9144 }),
        ("mi",  UnitDef { name: "mi",  category: Category::Length, factor: 1609.344 }),
        ("nmi", UnitDef { name: "nmi", category: Category::Length, factor: 1852.0 }),

        // ── mass (base: kg) ─────────────────────────────────────────────────
        ("mg", UnitDef { name: "mg", category: Category::Mass, factor: 1e-6 }),
        ("g",  UnitDef { name: "g",  category: Category::Mass, factor: 1e-3 }),
        ("kg", UnitDef { name: "kg", category: Category::Mass, factor: 1.0 }),
        ("t",  UnitDef { name: "t",  category: Category::Mass, factor: 1000.0 }),
        ("oz", UnitDef { name: "oz", category: Category::Mass, factor: 0.028349523125 }),
        ("lb", UnitDef { name: "lb", category: Category::Mass, factor: 0.45359237 }),
        ("st", UnitDef { name: "st", category: Category::Mass, factor: 6.35029318 }),

        // ── volume (base: l) ────────────────────────────────────────────────
        ("ml",   UnitDef { name: "ml",   category: Category::Volume, factor: 0.001 }),
        ("l",    UnitDef { name: "l",    category: Category::Volume, factor: 1.0 }),
        ("m3",   UnitDef { name: "m3",   category: Category::Volume, factor: 1000.0 }),
        ("tsp",  UnitDef { name: "tsp",  category: Category::Volume, factor: 0.00492892 }),
        ("tbsp", UnitDef { name: "tbsp", category: Category::Volume, factor: 0.01478676 }),
        ("cup",  UnitDef { name: "cup",  category: Category::Volume, factor: 0.2365882 }),
        ("pt",   UnitDef { name: "pt",   category: Category::Volume, factor: 0.4731765 }),
        ("qt",   UnitDef { name: "qt",   category: Category::Volume, factor: 0.9463529 }),
        ("gal",  UnitDef { name: "gal",  category: Category::Volume, factor: 3.7854118 }),

        // ── time (base: s) ──────────────────────────────────────────────────
        ("ms",  UnitDef { name: "ms",  category: Category::Time, factor: 0.001 }),
        ("s",   UnitDef { name: "s",   category: Category::Time, factor: 1.0 }),
        ("min", UnitDef { name: "min", category: Category::Time, factor: 60.0 }),
        ("h",   UnitDef { name: "h",   category: Category::Time, factor: 3600.0 }),
        ("d",   UnitDef { name: "d",   category: Category::Time, factor: 86400.0 }),
        ("wk",  UnitDef { name: "wk",  category: Category::Time, factor: 604800.0 }),
        ("mo",  UnitDef { name: "mo",  category: Category::Time, factor: 2592000.0 }),   // 30d
        ("y",   UnitDef { name: "y",   category: Category::Time, factor: 31536000.0 }),  // 365d

        // ── area (base: m2) ─────────────────────────────────────────────────
        ("cm2", UnitDef { name: "cm2", category: Category::Area, factor: 0.0001 }),
        ("m2",  UnitDef { name: "m2",  category: Category::Area, factor: 1.0 }),
        ("km2", UnitDef { name: "km2", category: Category::Area, factor: 1_000_000.0 }),
        ("in2", UnitDef { name: "in2", category: Category::Area, factor: 0.00064516 }),
        ("ft2", UnitDef { name: "ft2", category: Category::Area, factor: 0.09290304 }),
        ("ac",  UnitDef { name: "ac",  category: Category::Area, factor: 4046.8564224 }),
        ("ha",  UnitDef { name: "ha",  category: Category::Area, factor: 10000.0 }),

        // ── speed (base: m/s) ───────────────────────────────────────────────
        ("m/s",  UnitDef { name: "m/s",  category: Category::Speed, factor: 1.0 }),
        ("km/h", UnitDef { name: "km/h", category: Category::Speed, factor: 1.0 / 3.6 }),
        ("mph",  UnitDef { name: "mph",  category: Category::Speed, factor: 0.44704 }),
        ("kn",   UnitDef { name: "kn",   category: Category::Speed, factor: 0.514444 }),
        ("fps",  UnitDef { name: "fps",  category: Category::Speed, factor: 0.3048 }),

        // ── pressure (base: pa) ─────────────────────────────────────────────
        ("pa",   UnitDef { name: "pa",   category: Category::Pressure, factor: 1.0 }),
        ("kpa",  UnitDef { name: "kpa",  category: Category::Pressure, factor: 1000.0 }),
        ("bar",  UnitDef { name: "bar",  category: Category::Pressure, factor: 100_000.0 }),
        ("atm",  UnitDef { name: "atm",  category: Category::Pressure, factor: 101_325.0 }),
        ("psi",  UnitDef { name: "psi",  category: Category::Pressure, factor: 6894.757293168 }),
        ("mmHg", UnitDef { name: "mmHg", category: Category::Pressure, factor: 133.322387415 }),

        // ── energy (base: j) ────────────────────────────────────────────────
        ("j",    UnitDef { name: "j",    category: Category::Energy, factor: 1.0 }),
        ("kj",   UnitDef { name: "kj",   category: Category::Energy, factor: 1000.0 }),
        ("cal",  UnitDef { name: "cal",  category: Category::Energy, factor: 4.184 }),
        ("kcal", UnitDef { name: "kcal", category: Category::Energy, factor: 4184.0 }),
        ("wh",   UnitDef { name: "wh",   category: Category::Energy, factor: 3600.0 }),
        ("kwh",  UnitDef { name: "kwh",  category: Category::Energy, factor: 3_600_000.0 }),
        ("btu",  UnitDef { name: "btu",  category: Category::Energy, factor: 1055.05585262 }),

        // ── data (base: byte = b) ───────────────────────────────────────────
        // `b` -> byte (per spec tiebreaker); `B` also accepted.
        ("b",   UnitDef { name: "b",   category: Category::Data, factor: 1.0 }),
        ("kb",  UnitDef { name: "kb",  category: Category::Data, factor: 1000.0 }),
        ("mb",  UnitDef { name: "mb",  category: Category::Data, factor: 1_000_000.0 }),
        ("gb",  UnitDef { name: "gb",  category: Category::Data, factor: 1_000_000_000.0 }),
        ("tb",  UnitDef { name: "tb",  category: Category::Data, factor: 1_000_000_000_000.0 }),
        ("kib", UnitDef { name: "kib", category: Category::Data, factor: 1024.0 }),
        ("mib", UnitDef { name: "mib", category: Category::Data, factor: 1024.0 * 1024.0 }),
        ("gib", UnitDef { name: "gib", category: Category::Data, factor: 1024.0 * 1024.0 * 1024.0 }),
        ("tib", UnitDef { name: "tib", category: Category::Data, factor: 1024.0 * 1024.0 * 1024.0 * 1024.0 }),

        // ── temperature (factor unused; handled via formulas) ───────────────
        ("c", UnitDef { name: "c", category: Category::Temperature, factor: 0.0 }),
        ("f", UnitDef { name: "f", category: Category::Temperature, factor: 0.0 }),
        ("k", UnitDef { name: "k", category: Category::Temperature, factor: 0.0 }),
        ("r", UnitDef { name: "r", category: Category::Temperature, factor: 0.0 }),
    ]
}

/// Look up a unit string, with both exact and lowercased fallbacks.
fn lookup(s: &str) -> Option<&'static UnitDef> {
    let table = units();
    // exact match
    if let Some((_, def)) = table.iter().find(|(k, _)| *k == s) {
        return Some(def);
    }
    // lowercase fallback (handles KG, MPH, etc.)
    let lower = s.to_lowercase();
    if let Some((_, def)) = table.iter().find(|(k, _)| *k == lower) {
        return Some(def);
    }
    // common aliases
    let alias = match lower.as_str() {
        "litre" | "litres" | "liter" | "liters" => "l",
        "metre" | "metres" | "meter" | "meters" => "m",
        "kilometre" | "kilometres" | "kilometer" | "kilometers" => "km",
        "celsius" => "c",
        "fahrenheit" => "f",
        "kelvin" => "k",
        "rankine" => "r",
        "pound" | "pounds" => "lb",
        "ounce" | "ounces" => "oz",
        "gram" | "grams" => "g",
        "kilogram" | "kilograms" => "kg",
        "ton" | "tons" | "tonne" | "tonnes" => "t",
        "inch" | "inches" => "in",
        "foot" | "feet" => "ft",
        "yard" | "yards" => "yd",
        "mile" | "miles" => "mi",
        "byte" | "bytes" => "b",
        "mmhg" => "mmHg",
        _ => "",
    };
    if !alias.is_empty() {
        if let Some((_, def)) = table.iter().find(|(k, _)| *k == alias) {
            return Some(def);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Temperature handling
// ---------------------------------------------------------------------------

/// Convert a temperature from `from` unit to Celsius.
fn temp_to_c(v: f64, from: &str) -> f64 {
    match from {
        "c" => v,
        "f" => (v - 32.0) * 5.0 / 9.0,
        "k" => v - 273.15,
        "r" => (v - 491.67) * 5.0 / 9.0,
        _ => f64::NAN,
    }
}

/// Convert a temperature in Celsius to `to` unit.
fn temp_from_c(v: f64, to: &str) -> f64 {
    match to {
        "c" => v,
        "f" => v * 9.0 / 5.0 + 32.0,
        "k" => v + 273.15,
        "r" => (v + 273.15) * 9.0 / 5.0,
        _ => f64::NAN,
    }
}

// ---------------------------------------------------------------------------
// Parsing input value+unit
// ---------------------------------------------------------------------------

/// Parse a value+unit string like "100kg" or "100 kg" or "100" (no unit).
fn parse_value_unit(input: &str) -> Result<(f64, String), Error> {
    let s = input.trim();
    // Walk from the start collecting a numeric prefix: optional sign, digits,
    // optional decimal point, more digits, optional exponent.
    let bytes = s.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        // Only consume an exponent if followed by an actual digit (with
        // optional sign). Otherwise this 'e' belongs to the unit (rare).
        let mut j = i + 1;
        if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
            j += 1;
        }
        if j < bytes.len() && bytes[j].is_ascii_digit() {
            i = j;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
    }

    if i == 0 || (i == 1 && (bytes[0] == b'+' || bytes[0] == b'-')) {
        return Err(Error::Input(format!(
            "could not parse a number from '{s}'"
        )));
    }

    let num_str = &s[..i];
    let value: f64 = num_str
        .parse()
        .map_err(|_| Error::Input(format!("invalid number '{num_str}'")))?;
    let unit = s[i..].trim().to_string();
    Ok((value, unit))
}

// ---------------------------------------------------------------------------
// Conversion + formatting
// ---------------------------------------------------------------------------

fn convert(value: f64, from: &UnitDef, to: &UnitDef) -> f64 {
    if from.category != to.category {
        return f64::NAN;
    }
    if from.category == Category::Temperature {
        let c = temp_to_c(value, from.name);
        return temp_from_c(c, to.name);
    }
    // linear: base = value * from.factor; result = base / to.factor
    let base = value * from.factor;
    base / to.factor
}

/// Format a number for display: drop trailing zeros, switch to exponential
/// notation for very large/small magnitudes.
fn fmt_num(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    if !v.is_finite() {
        return format!("{v}");
    }
    let abs = v.abs();
    if abs < 1e-4 || abs >= 1e12 {
        let s = format!("{:.4e}", v);
        return s;
    }
    // up to ~8 significant digits, trim trailing zeros
    let mut s = format!("{:.8}", v);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

// ---------------------------------------------------------------------------
// run
// ---------------------------------------------------------------------------

pub fn run(value: &str, targets: &[String], json: bool) -> Result<(), Error> {
    let (num, unit_str) = parse_value_unit(value)?;
    if unit_str.is_empty() {
        return Err(Error::Usage(
            "no source unit given; example: '100 kg' or '100kg'".into(),
        ));
    }

    let from = lookup(&unit_str)
        .ok_or_else(|| Error::Input(format!("unknown unit '{unit_str}'")))?;

    // Resolve the target list. If empty, all units in the same category.
    let table = units();
    let resolved: Vec<&'static UnitDef> = if targets.is_empty() {
        table
            .iter()
            .filter_map(|(_, def)| {
                if def.category == from.category {
                    Some(def)
                } else {
                    None
                }
            })
            .collect()
    } else {
        let mut out: Vec<&'static UnitDef> = Vec::with_capacity(targets.len());
        for t in targets {
            let def = lookup(t)
                .ok_or_else(|| Error::Input(format!("unknown unit '{t}'")))?;
            if def.category != from.category {
                return Err(Error::Usage(format!(
                    "target unit '{t}' is in category '{}' but source '{}' is in category '{}'",
                    def.category.name(),
                    from.name,
                    from.category.name()
                )));
            }
            out.push(def);
        }
        out
    };

    if json {
        let mut obj = serde_json::Map::new();
        for def in &resolved {
            obj.insert(def.name.to_string(), json!(convert(num, from, def)));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::Value::Object(obj)).unwrap()
        );
        return Ok(());
    }

    // Single target → bare value
    if resolved.len() == 1 {
        println!("{}", fmt_num(convert(num, from, resolved[0])));
        return Ok(());
    }

    // Multiple → label per line. Spec example: "<target>: <value> <unit>".
    let width = resolved.iter().map(|d| d.name.len()).max().unwrap_or(3);
    for def in &resolved {
        let v = convert(num, from, def);
        println!(
            "{:<width$}: {} {}",
            def.name,
            fmt_num(v),
            def.name,
            width = width
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn parse_value_unit_concat() {
        let (v, u) = parse_value_unit("100kg").unwrap();
        assert_eq!(v, 100.0);
        assert_eq!(u, "kg");
    }

    #[test]
    fn parse_value_unit_spaced() {
        let (v, u) = parse_value_unit("100 kg").unwrap();
        assert_eq!(v, 100.0);
        assert_eq!(u, "kg");
    }

    #[test]
    fn parse_value_unit_decimal() {
        let (v, u) = parse_value_unit("12.5lb").unwrap();
        assert_eq!(v, 12.5);
        assert_eq!(u, "lb");
    }

    #[test]
    fn parse_value_unit_negative() {
        let (v, u) = parse_value_unit("-40c").unwrap();
        assert_eq!(v, -40.0);
        assert_eq!(u, "c");
    }

    #[test]
    fn parse_value_unit_no_unit() {
        let (v, u) = parse_value_unit("42").unwrap();
        assert_eq!(v, 42.0);
        assert_eq!(u, "");
    }

    #[test]
    fn unit_length_basic() {
        let from = lookup("km").unwrap();
        let to = lookup("m").unwrap();
        assert!(approx(convert(1.0, from, to), 1000.0, 1e-9));
    }

    #[test]
    fn unit_mass_kg_to_lb() {
        let from = lookup("kg").unwrap();
        let to = lookup("lb").unwrap();
        assert!(approx(convert(100.0, from, to), 220.462, 0.01));
    }

    #[test]
    fn unit_volume_l_to_gal() {
        let from = lookup("l").unwrap();
        let to = lookup("gal").unwrap();
        assert!(approx(convert(3.7854118, from, to), 1.0, 1e-3));
    }

    #[test]
    fn unit_time_h_to_s() {
        let from = lookup("h").unwrap();
        let to = lookup("s").unwrap();
        assert!(approx(convert(1.0, from, to), 3600.0, 1e-9));
    }

    #[test]
    fn unit_area_ha_to_m2() {
        let from = lookup("ha").unwrap();
        let to = lookup("m2").unwrap();
        assert!(approx(convert(1.0, from, to), 10000.0, 1e-6));
    }

    #[test]
    fn unit_speed_kmh_to_mph() {
        let from = lookup("km/h").unwrap();
        let to = lookup("mph").unwrap();
        assert!(approx(convert(100.0, from, to), 62.137, 0.01));
    }

    #[test]
    fn unit_pressure_atm_to_pa() {
        let from = lookup("atm").unwrap();
        let to = lookup("pa").unwrap();
        assert!(approx(convert(1.0, from, to), 101325.0, 1e-6));
    }

    #[test]
    fn unit_energy_kcal_to_j() {
        let from = lookup("kcal").unwrap();
        let to = lookup("j").unwrap();
        assert!(approx(convert(1.0, from, to), 4184.0, 1e-6));
    }

    #[test]
    fn unit_data_kib_to_b() {
        let from = lookup("kib").unwrap();
        let to = lookup("b").unwrap();
        assert!(approx(convert(1.0, from, to), 1024.0, 1e-9));
    }

    #[test]
    fn unit_temp_c_to_f() {
        let from = lookup("c").unwrap();
        let to = lookup("f").unwrap();
        assert!(approx(convert(100.0, from, to), 212.0, 1e-6));
        assert!(approx(convert(0.0, from, to), 32.0, 1e-6));
    }

    #[test]
    fn unit_temp_f_to_c() {
        let from = lookup("f").unwrap();
        let to = lookup("c").unwrap();
        assert!(approx(convert(212.0, from, to), 100.0, 1e-6));
    }

    #[test]
    fn unit_temp_c_to_k() {
        let from = lookup("c").unwrap();
        let to = lookup("k").unwrap();
        assert!(approx(convert(0.0, from, to), 273.15, 1e-6));
    }

    #[test]
    fn unit_temp_r_to_f() {
        // 491.67 R = 32 F (freezing)
        let from = lookup("r").unwrap();
        let to = lookup("f").unwrap();
        assert!(approx(convert(491.67, from, to), 32.0, 1e-3));
    }

    #[test]
    fn unit_cross_category_errors() {
        let r = run("100", &[String::from("kg")], false);
        assert!(r.is_err());
        let r = run(
            "100kg",
            &[String::from("m")],
            false,
        );
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn unit_unknown_unit_errors() {
        let r = run("100xyz", &[], false);
        assert!(matches!(r, Err(Error::Input(_))));
    }

    #[test]
    fn unit_lookup_aliases() {
        assert_eq!(lookup("celsius").unwrap().name, "c");
        assert_eq!(lookup("KG").unwrap().name, "kg");
        assert_eq!(lookup("MPH").unwrap().name, "mph");
    }

    #[test]
    fn unit_run_single_target() {
        run("100kg", &[String::from("lb")], false).unwrap();
        run("100 c", &[String::from("f")], false).unwrap();
    }

    #[test]
    fn unit_run_default_lists_category() {
        run("1m", &[], false).unwrap();
    }

    #[test]
    fn unit_run_json() {
        run("100kg", &[String::from("lb")], true).unwrap();
    }
}
