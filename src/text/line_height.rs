use crate::error::Error;
use serde_json::json;

pub struct LineHeightRatio {
    pub name: &'static str,
    pub ratio: f64,
}

pub const RATIOS: &[LineHeightRatio] = &[
    LineHeightRatio { name: "tight",   ratio: 1.2   },
    LineHeightRatio { name: "snug",    ratio: 1.375 },
    LineHeightRatio { name: "normal",  ratio: 1.5   },
    LineHeightRatio { name: "relaxed", ratio: 1.625 },
    LineHeightRatio { name: "loose",   ratio: 2.0   },
    LineHeightRatio { name: "golden",  ratio: 1.618 },
];

pub fn run(font_size: f64, filter: Option<&str>, as_json: bool) -> Result<(), Error> {
    if let Some(name) = filter {
        let entry = RATIOS
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| {
                Error::Usage(format!(
                    "unknown line-height name '{name}'; valid names: tight, snug, normal, relaxed, loose, golden"
                ))
            })?;

        let px = font_size * entry.ratio;
        if as_json {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "name":  entry.name,
                    "ratio": entry.ratio,
                    "px":    round1(px),
                }))
                .unwrap()
            );
        } else {
            println!("{:.1}px", px);
        }
        return Ok(());
    }

    if as_json {
        let items: Vec<_> = RATIOS
            .iter()
            .map(|r| {
                let px = font_size * r.ratio;
                json!({
                    "name":  r.name,
                    "ratio": r.ratio,
                    "px":    round1(px),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items).unwrap());
    } else {
        for r in RATIOS {
            let px = font_size * r.ratio;
            // "tight:    1.200  (19.2px)"
            // name field: 8 chars left-aligned, ratio: 7 chars (1 decimal + 3 fraction)
            println!("{:<8} {:.3}  ({:.1}px)", format!("{}:", r.name), r.ratio, px);
        }
    }

    Ok(())
}

/// Round to one decimal place (avoids floating-point noise in JSON output).
fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ratios_present() {
        assert_eq!(RATIOS.len(), 6);
        let names: Vec<_> = RATIOS.iter().map(|r| r.name).collect();
        assert_eq!(names, ["tight", "snug", "normal", "relaxed", "loose", "golden"]);
    }

    #[test]
    fn ratio_values() {
        assert_eq!(RATIOS[0].ratio, 1.2);
        assert_eq!(RATIOS[1].ratio, 1.375);
        assert_eq!(RATIOS[2].ratio, 1.5);
        assert_eq!(RATIOS[3].ratio, 1.625);
        assert_eq!(RATIOS[4].ratio, 2.0);
        assert_eq!(RATIOS[5].ratio, 1.618);
    }

    #[test]
    fn computed_px_at_16() {
        let size = 16.0_f64;
        assert!((RATIOS[0].ratio * size - 19.2).abs() < 1e-9); // tight
        assert!((RATIOS[1].ratio * size - 22.0).abs() < 1e-9); // snug
        assert!((RATIOS[2].ratio * size - 24.0).abs() < 1e-9); // normal
        assert!((RATIOS[3].ratio * size - 26.0).abs() < 1e-9); // relaxed
        assert!((RATIOS[4].ratio * size - 32.0).abs() < 1e-9); // loose
        assert!((RATIOS[5].ratio * size - 25.888).abs() < 1e-9); // golden
    }

    #[test]
    fn run_all_text_does_not_panic() {
        run(16.0, None, false).unwrap();
    }

    #[test]
    fn run_all_json_does_not_panic() {
        run(16.0, None, true).unwrap();
    }

    #[test]
    fn run_single_filter() {
        run(16.0, Some("golden"), false).unwrap();
        run(16.0, Some("tight"), false).unwrap();
    }

    #[test]
    fn run_filter_case_insensitive() {
        run(16.0, Some("NORMAL"), false).unwrap();
        run(16.0, Some("Loose"), false).unwrap();
    }

    #[test]
    fn run_unknown_filter_errors() {
        let result = run(16.0, Some("massive"), false);
        assert!(result.is_err());
        match result {
            Err(Error::Usage(msg)) => assert!(msg.contains("massive")),
            _ => panic!("expected Usage error"),
        }
    }

    #[test]
    fn round1_correctness() {
        assert_eq!(round1(19.2), 19.2);
        assert_eq!(round1(25.888), 25.9);
        assert_eq!(round1(32.0), 32.0);
    }
}
