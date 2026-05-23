use crate::colour::{oklab_to_oklch, oklab_to_srgb, oklch_to_oklab, srgb_to_oklab, Colour};
use crate::error::Error;
use crate::output;
use serde_json::json;

// ---------------------------------------------------------------------------
// Shade mode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShadeMode {
    Classic,
    Vivid,
    Muted,
    HueShift,
}

impl ShadeMode {
    pub fn parse(s: &str) -> Result<Self, Error> {
        match s.to_lowercase().as_str() {
            "classic" => Ok(ShadeMode::Classic),
            "vivid" => Ok(ShadeMode::Vivid),
            "muted" => Ok(ShadeMode::Muted),
            "hue-shift" | "hueshift" | "hue_shift" => Ok(ShadeMode::HueShift),
            other => Err(Error::Usage(format!(
                "unknown shade mode: {other}\n\
                 valid modes: classic, vivid, muted, hue-shift"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The 11 Tailwind shade stops.
pub const STOPS: [u32; 11] = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950];

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Linear interpolation between `a` and `b` by factor `t` (0.0–1.0).
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Bell curve centred at t=0.5, with peak value `peak` and min value `min_val`.
/// Uses a parabola: value = peak - (peak - min_val) * 4 * (t - 0.5)^2
fn bell(t: f64, peak: f64, min_val: f64) -> f64 {
    let deviation = (t - 0.5) * (t - 0.5);
    peak - (peak - min_val) * 4.0 * deviation
}

/// Compute the chroma scale factor for a given mode and shade position `t`.
fn chroma_factor(mode: ShadeMode, t: f64) -> f64 {
    match mode {
        ShadeMode::Classic => bell(t, 1.0, 0.0).max(0.0),
        ShadeMode::Vivid => bell(t, 1.2, 0.0).max(0.0),
        ShadeMode::Muted => bell(t, 0.7, 0.0).max(0.0),
        ShadeMode::HueShift => bell(t, 1.0, 0.0).max(0.0),
    }
}

/// Compute hue offset for a given mode and shade position `t`.
fn hue_offset(mode: ShadeMode, t: f64) -> f64 {
    match mode {
        ShadeMode::HueShift => (t - 0.5) * 30.0,
        _ => 0.0,
    }
}

/// Generate 11 Tailwind shades for `base` using the given mode.
/// Returns pairs of (stop, Colour).
pub fn compute(base: Colour, mode: ShadeMode) -> Vec<(u32, Colour)> {
    let (lab_l, lab_a, lab_b) = srgb_to_oklab(base.r, base.g, base.b);
    let (_base_l, base_c, base_h) = oklab_to_oklch(lab_l, lab_a, lab_b);

    STOPS
        .iter()
        .map(|&stop| {
            let t = stop as f64 / 1000.0;

            let target_l = lerp(0.97, 0.15, t);
            let target_c = base_c * chroma_factor(mode, t);
            let target_h = base_h + hue_offset(mode, t);

            let (new_lab_l, new_lab_a, new_lab_b) = oklch_to_oklab(target_l, target_c, target_h);
            let (r, g, b) = oklab_to_srgb(new_lab_l, new_lab_a, new_lab_b);

            let colour = Colour::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
            (stop, colour)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(
    colour_input: &str,
    mode_input: Option<&str>,
    as_json: bool,
    pretty: bool,
) -> Result<(), Error> {
    let base = Colour::parse(colour_input)?;
    let mode = match mode_input {
        Some(s) => ShadeMode::parse(s)?,
        None => ShadeMode::Classic,
    };

    let shades = compute(base, mode);

    if as_json {
        let mut map = serde_json::Map::new();
        for (stop, colour) in &shades {
            map.insert(stop.to_string(), json!(colour.to_hex()));
        }
        println!("{}", serde_json::to_string_pretty(&map).unwrap());
    } else if pretty {
        output_pretty(&shades);
    } else {
        for (stop, colour) in &shades {
            println!("{stop}: {}", colour.to_hex());
        }
    }

    Ok(())
}

fn output_pretty(shades: &[(u32, Colour)]) {
    for (stop, colour) in shades {
        let (r, g, b) = colour.to_u8();
        let block = output::colour_block(r, g, b);
        if block.is_empty() {
            println!("{stop:>4}: {}", colour.to_hex());
        } else {
            println!("{stop:>4}: {block} {}", colour.to_hex());
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Colour {
        Colour::parse(s).unwrap()
    }

    // --- ShadeMode::parse ---

    #[test]
    fn parse_mode_classic() {
        assert_eq!(ShadeMode::parse("classic").unwrap(), ShadeMode::Classic);
        assert_eq!(ShadeMode::parse("CLASSIC").unwrap(), ShadeMode::Classic);
    }

    #[test]
    fn parse_mode_vivid() {
        assert_eq!(ShadeMode::parse("vivid").unwrap(), ShadeMode::Vivid);
    }

    #[test]
    fn parse_mode_muted() {
        assert_eq!(ShadeMode::parse("muted").unwrap(), ShadeMode::Muted);
    }

    #[test]
    fn parse_mode_hue_shift() {
        assert_eq!(ShadeMode::parse("hue-shift").unwrap(), ShadeMode::HueShift);
        assert_eq!(ShadeMode::parse("hueshift").unwrap(), ShadeMode::HueShift);
        assert_eq!(ShadeMode::parse("hue_shift").unwrap(), ShadeMode::HueShift);
    }

    #[test]
    fn parse_mode_invalid() {
        assert!(ShadeMode::parse("rainbow").is_err());
        assert!(ShadeMode::parse("").is_err());
    }

    // --- compute: structure ---

    #[test]
    fn compute_returns_eleven_shades() {
        let shades = compute(parse("#3b82f6"), ShadeMode::Classic);
        assert_eq!(shades.len(), 11);
    }

    #[test]
    fn compute_stops_are_correct() {
        let shades = compute(parse("#3b82f6"), ShadeMode::Classic);
        let stops: Vec<u32> = shades.iter().map(|(s, _)| *s).collect();
        assert_eq!(stops, vec![50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950]);
    }

    // --- compute: lightness ordering ---

    #[test]
    fn shades_lightest_to_darkest() {
        // Shade 50 should be much lighter than shade 950
        let shades = compute(parse("#3b82f6"), ShadeMode::Classic);
        let shade_50 = shades.iter().find(|(s, _)| *s == 50).unwrap().1;
        let shade_950 = shades.iter().find(|(s, _)| *s == 950).unwrap().1;

        let (lab_l_50, _, _) = srgb_to_oklab(shade_50.r, shade_50.g, shade_50.b);
        let (lab_l_950, _, _) = srgb_to_oklab(shade_950.r, shade_950.g, shade_950.b);

        assert!(lab_l_50 > lab_l_950, "50 should be lighter than 950");
    }

    #[test]
    fn lightness_is_monotonically_decreasing() {
        let shades = compute(parse("#ff6600"), ShadeMode::Classic);
        let mut prev_l = f64::INFINITY;
        for (stop, colour) in &shades {
            let (l, _, _) = srgb_to_oklab(colour.r, colour.g, colour.b);
            assert!(l < prev_l, "lightness should decrease at stop {stop}");
            prev_l = l;
        }
    }

    // --- compute: colours are in valid sRGB range ---

    #[test]
    fn all_shades_are_valid_srgb() {
        for mode in [ShadeMode::Classic, ShadeMode::Vivid, ShadeMode::Muted, ShadeMode::HueShift] {
            let shades = compute(parse("#3b82f6"), mode);
            for (stop, colour) in &shades {
                assert!(colour.r >= 0.0 && colour.r <= 1.0, "r out of range at {stop}");
                assert!(colour.g >= 0.0 && colour.g <= 1.0, "g out of range at {stop}");
                assert!(colour.b >= 0.0 && colour.b <= 1.0, "b out of range at {stop}");
            }
        }
    }

    // --- compute: vivid has higher chroma than muted ---

    #[test]
    fn vivid_more_saturated_than_muted_at_midpoint() {
        let base = parse("#3b82f6");
        let vivid = compute(base, ShadeMode::Vivid);
        let muted = compute(base, ShadeMode::Muted);

        // At stop 500 (t=0.5, midpoint), vivid should be more saturated
        let vivid_500 = vivid.iter().find(|(s, _)| *s == 500).unwrap().1;
        let muted_500 = muted.iter().find(|(s, _)| *s == 500).unwrap().1;

        let (lab_l_v, lab_a_v, lab_b_v) = srgb_to_oklab(vivid_500.r, vivid_500.g, vivid_500.b);
        let (_, vivid_c, _) = oklab_to_oklch(lab_l_v, lab_a_v, lab_b_v);

        let (lab_l_m, lab_a_m, lab_b_m) = srgb_to_oklab(muted_500.r, muted_500.g, muted_500.b);
        let (_, muted_c, _) = oklab_to_oklch(lab_l_m, lab_a_m, lab_b_m);

        assert!(
            vivid_c > muted_c,
            "vivid chroma ({vivid_c:.4}) should be greater than muted ({muted_c:.4}) at 500"
        );
    }

    // --- compute: hue-shift changes hue at extremes ---

    #[test]
    fn hue_shift_differs_from_classic_at_extremes() {
        let base = parse("#3b82f6");
        let classic = compute(base, ShadeMode::Classic);
        let hue_shift = compute(base, ShadeMode::HueShift);

        let to_hue = |colour: Colour| {
            let (l, a, b) = srgb_to_oklab(colour.r, colour.g, colour.b);
            let (_, _, h) = oklab_to_oklch(l, a, b);
            h
        };

        // At 50 (t=0.05) hue-shift hue should differ from classic
        let classic_50 = classic.iter().find(|(s, _)| *s == 50).unwrap().1;
        let shift_50 = hue_shift.iter().find(|(s, _)| *s == 50).unwrap().1;

        let h_classic = to_hue(classic_50);
        let h_shift = to_hue(shift_50);

        // They should differ by a noticeable amount (the hue shift is (0.05-0.5)*30 = -13.5°)
        assert!((h_classic - h_shift).abs() > 1.0, "hue-shift should differ from classic at 50");
    }

    // --- run: smoke tests ---

    #[test]
    fn run_default_mode() {
        assert!(run("#3b82f6", None, false, false).is_ok());
    }

    #[test]
    fn run_vivid_mode() {
        assert!(run("#3b82f6", Some("vivid"), false, false).is_ok());
    }

    #[test]
    fn run_json_output() {
        assert!(run("#ff6600", Some("classic"), true, false).is_ok());
    }

    #[test]
    fn run_invalid_colour() {
        assert!(run("not-a-colour", None, false, false).is_err());
    }

    #[test]
    fn run_invalid_mode() {
        assert!(run("#3b82f6", Some("neon"), false, false).is_err());
    }

    // --- bell curve ---

    #[test]
    fn bell_peaks_at_midpoint() {
        let mid = bell(0.5, 1.0, 0.0);
        let low = bell(0.1, 1.0, 0.0);
        let high = bell(0.9, 1.0, 0.0);
        assert!(mid > low, "bell should peak at t=0.5");
        assert!(mid > high, "bell should peak at t=0.5");
    }

    #[test]
    fn bell_peak_value_at_midpoint() {
        let v = bell(0.5, 1.0, 0.0);
        assert!((v - 1.0).abs() < 1e-10, "bell(0.5) should equal peak");
    }

    // --- lerp ---

    #[test]
    fn lerp_endpoints() {
        assert!((lerp(0.97, 0.15, 0.0) - 0.97).abs() < 1e-10);
        assert!((lerp(0.97, 0.15, 1.0) - 0.15).abs() < 1e-10);
    }

    #[test]
    fn lerp_midpoint() {
        let mid = lerp(0.0, 1.0, 0.5);
        assert!((mid - 0.5).abs() < 1e-10);
    }
}
