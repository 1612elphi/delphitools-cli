use crate::colour::{hsl_to_rgb, rgb_to_hsl, Colour};
use crate::error::Error;
use crate::output;
use serde_json::json;

// ---------------------------------------------------------------------------
// Harmony types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HarmonyType {
    Complementary,
    Analogous,
    Triadic,
    Tetradic,
    Split,
}

impl HarmonyType {
    /// Hue offsets (degrees) that define the harmony partners.
    fn offsets(self) -> &'static [i32] {
        match self {
            HarmonyType::Complementary => &[180],
            HarmonyType::Analogous => &[-30, 30],
            HarmonyType::Triadic => &[120, 240],
            HarmonyType::Tetradic => &[90, 180, 270],
            HarmonyType::Split => &[150, 210],
        }
    }

    fn name(self) -> &'static str {
        match self {
            HarmonyType::Complementary => "complementary",
            HarmonyType::Analogous => "analogous",
            HarmonyType::Triadic => "triadic",
            HarmonyType::Tetradic => "tetradic",
            HarmonyType::Split => "split",
        }
    }

    fn parse(s: &str) -> Result<Self, Error> {
        match s.to_lowercase().as_str() {
            "complementary" | "comp" => Ok(HarmonyType::Complementary),
            "analogous" | "analog" => Ok(HarmonyType::Analogous),
            "triadic" | "triad" => Ok(HarmonyType::Triadic),
            "tetradic" | "tetrad" => Ok(HarmonyType::Tetradic),
            "split" | "split-complementary" => Ok(HarmonyType::Split),
            other => Err(Error::Usage(format!(
                "unknown harmony type: {other}\n\
                 valid types: complementary, analogous, triadic, tetradic, split"
            ))),
        }
    }

    fn all() -> &'static [HarmonyType] {
        &[
            HarmonyType::Complementary,
            HarmonyType::Analogous,
            HarmonyType::Triadic,
            HarmonyType::Tetradic,
            HarmonyType::Split,
        ]
    }
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Returns the base colour followed by all harmony partner colours.
pub fn compute(base: Colour, harmony: HarmonyType) -> Vec<Colour> {
    let (h, s, l) = rgb_to_hsl(base.r, base.g, base.b);
    let mut result = vec![base];
    for &offset in harmony.offsets() {
        let new_h = ((h + offset as f64) % 360.0 + 360.0) % 360.0;
        let (r, g, b) = hsl_to_rgb(new_h, s, l);
        result.push(Colour::new(r, g, b));
    }
    result
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(
    colour_input: &str,
    harmony_type: Option<&str>,
    as_json: bool,
    pretty: bool,
) -> Result<(), Error> {
    let base = Colour::parse(colour_input)?;

    if let Some(type_str) = harmony_type {
        let harmony = HarmonyType::parse(type_str)?;
        let colours = compute(base, harmony);
        output_harmony(harmony.name(), &colours, as_json, pretty);
    } else {
        // Show all harmony types
        if as_json {
            let mut map = serde_json::Map::new();
            for &harmony in HarmonyType::all() {
                let colours = compute(base, harmony);
                let hexes: Vec<serde_json::Value> =
                    colours.iter().map(|c| json!(c.to_hex())).collect();
                map.insert(harmony.name().to_string(), json!(hexes));
            }
            println!("{}", serde_json::to_string_pretty(&map).unwrap());
        } else if pretty {
            for &harmony in HarmonyType::all() {
                let colours = compute(base, harmony);
                output_pretty(harmony.name(), &colours);
            }
        } else {
            for &harmony in HarmonyType::all() {
                let colours = compute(base, harmony);
                println!("{}:", harmony.name());
                for c in &colours {
                    println!("  {}", c.to_hex());
                }
            }
        }
    }

    Ok(())
}

fn output_harmony(name: &str, colours: &[Colour], as_json: bool, pretty: bool) {
    if as_json {
        let hexes: Vec<serde_json::Value> = colours.iter().map(|c| json!(c.to_hex())).collect();
        println!("{}", serde_json::to_string_pretty(&json!(hexes)).unwrap());
    } else if pretty {
        output_pretty(name, colours);
    } else {
        for c in colours {
            println!("{}", c.to_hex());
        }
    }
}

fn output_pretty(name: &str, colours: &[Colour]) {
    println!("{name}");
    let swatches: Vec<String> = colours
        .iter()
        .map(|c| {
            let (r, g, b) = c.to_u8();
            let block = output::colour_block(r, g, b);
            if block.is_empty() {
                c.to_hex()
            } else {
                format!("{block} {}", c.to_hex())
            }
        })
        .collect();
    println!("  {}", swatches.join("  "));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.5 // tolerance in 0–255 space after u8 rounding
    }

    fn parse(s: &str) -> Colour {
        Colour::parse(s).unwrap()
    }

    // --- HarmonyType::parse ---

    #[test]
    fn parse_harmony_type_complementary() {
        assert_eq!(HarmonyType::parse("complementary").unwrap(), HarmonyType::Complementary);
        assert_eq!(HarmonyType::parse("comp").unwrap(), HarmonyType::Complementary);
    }

    #[test]
    fn parse_harmony_type_analogous() {
        assert_eq!(HarmonyType::parse("analogous").unwrap(), HarmonyType::Analogous);
        assert_eq!(HarmonyType::parse("analog").unwrap(), HarmonyType::Analogous);
    }

    #[test]
    fn parse_harmony_type_triadic() {
        assert_eq!(HarmonyType::parse("triadic").unwrap(), HarmonyType::Triadic);
        assert_eq!(HarmonyType::parse("triad").unwrap(), HarmonyType::Triadic);
    }

    #[test]
    fn parse_harmony_type_tetradic() {
        assert_eq!(HarmonyType::parse("tetradic").unwrap(), HarmonyType::Tetradic);
        assert_eq!(HarmonyType::parse("tetrad").unwrap(), HarmonyType::Tetradic);
    }

    #[test]
    fn parse_harmony_type_split() {
        assert_eq!(HarmonyType::parse("split").unwrap(), HarmonyType::Split);
        assert_eq!(HarmonyType::parse("split-complementary").unwrap(), HarmonyType::Split);
    }

    #[test]
    fn parse_harmony_type_case_insensitive() {
        assert_eq!(HarmonyType::parse("Complementary").unwrap(), HarmonyType::Complementary);
        assert_eq!(HarmonyType::parse("TRIADIC").unwrap(), HarmonyType::Triadic);
    }

    #[test]
    fn parse_harmony_type_invalid() {
        assert!(HarmonyType::parse("rainbow").is_err());
        assert!(HarmonyType::parse("").is_err());
    }

    // --- compute: result length ---

    #[test]
    fn complementary_has_two_colours() {
        let colours = compute(parse("#ff6600"), HarmonyType::Complementary);
        assert_eq!(colours.len(), 2);
    }

    #[test]
    fn analogous_has_three_colours() {
        let colours = compute(parse("#ff6600"), HarmonyType::Analogous);
        assert_eq!(colours.len(), 3);
    }

    #[test]
    fn triadic_has_three_colours() {
        let colours = compute(parse("#ff6600"), HarmonyType::Triadic);
        assert_eq!(colours.len(), 3);
    }

    #[test]
    fn tetradic_has_four_colours() {
        let colours = compute(parse("#ff6600"), HarmonyType::Tetradic);
        assert_eq!(colours.len(), 4);
    }

    #[test]
    fn split_has_three_colours() {
        let colours = compute(parse("#ff6600"), HarmonyType::Split);
        assert_eq!(colours.len(), 3);
    }

    // --- compute: base colour is first ---

    #[test]
    fn base_colour_is_first() {
        let base = parse("#ff6600");
        let colours = compute(base, HarmonyType::Complementary);
        assert_eq!(colours[0].to_hex(), base.to_hex());
    }

    // --- compute: complementary is 180° opposite ---

    #[test]
    fn red_complementary_is_cyan() {
        // pure red hsl(0, 100%, 50%) → complement is hsl(180, 100%, 50%) = cyan
        let colours = compute(parse("#ff0000"), HarmonyType::Complementary);
        let (r, g, b) = colours[1].to_u8();
        assert!(approx(r as f64, 0.0), "r should be ~0, got {r}");
        assert!(approx(g as f64, 255.0), "g should be ~255, got {g}");
        assert!(approx(b as f64, 255.0), "b should be ~255, got {b}");
    }

    #[test]
    fn complementary_hex_output() {
        let colours = compute(parse("#ff0000"), HarmonyType::Complementary);
        assert_eq!(colours[0].to_hex(), "#ff0000");
        assert_eq!(colours[1].to_hex(), "#00ffff");
    }

    // --- compute: triadic ---

    #[test]
    fn red_triadic_partners() {
        // red (0°) → 120° = green, 240° = blue
        let colours = compute(parse("#ff0000"), HarmonyType::Triadic);
        let (r1, g1, b1) = colours[1].to_u8();
        let (r2, g2, b2) = colours[2].to_u8();
        // 120°: pure green
        assert!(approx(r1 as f64, 0.0));
        assert!(approx(g1 as f64, 255.0));
        assert!(approx(b1 as f64, 0.0));
        // 240°: pure blue
        assert!(approx(r2 as f64, 0.0));
        assert!(approx(g2 as f64, 0.0));
        assert!(approx(b2 as f64, 255.0));
    }

    // --- compute: hue wraps correctly ---

    #[test]
    fn hue_wraps_at_360() {
        // blue at 240°, complement at 60° (yellow)
        let colours = compute(parse("#0000ff"), HarmonyType::Complementary);
        let (r, g, b) = colours[1].to_u8();
        assert!(approx(r as f64, 255.0), "r should be ~255, got {r}");
        assert!(approx(g as f64, 255.0), "g should be ~255, got {g}");
        assert!(approx(b as f64, 0.0), "b should be ~0, got {b}");
    }

    // --- compute: saturation and lightness preserved ---

    #[test]
    fn saturation_and_lightness_preserved() {
        let base = parse("#ff6600"); // a vivid orange
        let (_, s_base, l_base) = rgb_to_hsl(base.r, base.g, base.b);
        for colour in compute(base, HarmonyType::Tetradic).iter().skip(1) {
            let (_, s, l) = rgb_to_hsl(colour.r, colour.g, colour.b);
            assert!((s - s_base).abs() < 0.01, "saturation drifted");
            assert!((l - l_base).abs() < 0.01, "lightness drifted");
        }
    }

    // --- run: plain output ---

    #[test]
    fn run_plain_single_type() {
        // Smoke test: run should not error
        assert!(run("#ff0000", Some("complementary"), false, false).is_ok());
    }

    #[test]
    fn run_plain_all_types() {
        assert!(run("#ff0000", None, false, false).is_ok());
    }

    #[test]
    fn run_json_single_type() {
        assert!(run("#ff0000", Some("triadic"), true, false).is_ok());
    }

    #[test]
    fn run_json_all_types() {
        assert!(run("#ff0000", None, true, false).is_ok());
    }

    #[test]
    fn run_invalid_colour() {
        assert!(run("not-a-colour", Some("complementary"), false, false).is_err());
    }

    #[test]
    fn run_invalid_harmony_type() {
        assert!(run("#ff0000", Some("rainbow"), false, false).is_err());
    }
}
