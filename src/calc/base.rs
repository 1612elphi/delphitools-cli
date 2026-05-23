use crate::error::Error;
use serde_json::json;

/// All bases we know about, in display order.
const ALL_BASES: &[&str] = &["dec", "hex", "oct", "bin"];

/// Parse the input string as an integer, with optional explicit base or auto-detection.
/// Auto-detection recognises 0x/0X (hex), 0b/0B (bin), 0o/0O (oct).
fn parse_value(input: &str, from: &str) -> Result<u128, Error> {
    let input = input.trim();
    // If from == "auto" or "dec" (the default), check for prefix
    let (s, radix) = match from {
        "hex" => (input, 16u32),
        "bin" => (input, 2),
        "oct" => (input, 8),
        _ => {
            // auto / dec: detect prefix
            if let Some(rest) = input.strip_prefix("0x").or_else(|| input.strip_prefix("0X")) {
                (rest, 16)
            } else if let Some(rest) =
                input.strip_prefix("0b").or_else(|| input.strip_prefix("0B"))
            {
                (rest, 2)
            } else if let Some(rest) =
                input.strip_prefix("0o").or_else(|| input.strip_prefix("0O"))
            {
                (rest, 8)
            } else {
                (input, 10)
            }
        }
    };

    u128::from_str_radix(s, radix).map_err(|_| {
        Error::Input(format!(
            "could not parse '{input}' as a base-{radix} number"
        ))
    })
}

fn render(value: u128, base: &str) -> String {
    match base {
        "hex" => format!("{:X}", value),
        "oct" => format!("{:o}", value),
        "bin" => format!("{:b}", value),
        _ => format!("{}", value),
    }
}

pub fn run(
    input: &str,
    targets: &[String],
    from: &str,
    as_json: bool,
) -> Result<(), Error> {
    // Validate 'from'
    if !["dec", "hex", "oct", "bin", "auto"].contains(&from) {
        return Err(Error::Usage(format!(
            "unknown base '{from}'; valid bases: dec, hex, oct, bin"
        )));
    }

    let value = parse_value(input, from)?;

    // Validate targets
    for t in targets {
        if !ALL_BASES.contains(&t.as_str()) {
            return Err(Error::Usage(format!(
                "unknown target base '{t}'; valid bases: dec, hex, oct, bin"
            )));
        }
    }

    // Resolve which bases to display
    let display: Vec<&str> = if targets.is_empty() {
        ALL_BASES.to_vec()
    } else {
        targets.iter().map(|s| s.as_str()).collect()
    };

    if as_json {
        // Always emit all four bases in JSON, regardless of targets
        let obj = json!({
            "dec": render(value, "dec"),
            "hex": render(value, "hex"),
            "oct": render(value, "oct"),
            "bin": render(value, "bin"),
        });
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
        return Ok(());
    }

    // Single target → bare, pipeable output
    if display.len() == 1 {
        println!("{}", render(value, display[0]));
        return Ok(());
    }

    // Multiple targets → labeled
    let label_width = display.iter().map(|b| b.len()).max().unwrap_or(3);
    for base in &display {
        println!("{:<width$}: {}", base, render(value, base), width = label_width);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_value ---

    #[test]
    fn parse_decimal() {
        assert_eq!(parse_value("255", "dec").unwrap(), 255);
    }

    #[test]
    fn parse_hex_explicit() {
        assert_eq!(parse_value("ff", "hex").unwrap(), 255);
        assert_eq!(parse_value("FF", "hex").unwrap(), 255);
    }

    #[test]
    fn parse_oct_explicit() {
        assert_eq!(parse_value("377", "oct").unwrap(), 255);
    }

    #[test]
    fn parse_bin_explicit() {
        assert_eq!(parse_value("11111111", "bin").unwrap(), 255);
    }

    #[test]
    fn parse_auto_hex_prefix() {
        assert_eq!(parse_value("0xff", "auto").unwrap(), 255);
        assert_eq!(parse_value("0XFF", "auto").unwrap(), 255);
    }

    #[test]
    fn parse_auto_bin_prefix() {
        assert_eq!(parse_value("0b11111111", "auto").unwrap(), 255);
        assert_eq!(parse_value("0B11111111", "auto").unwrap(), 255);
    }

    #[test]
    fn parse_auto_oct_prefix() {
        assert_eq!(parse_value("0o377", "auto").unwrap(), 255);
        assert_eq!(parse_value("0O377", "auto").unwrap(), 255);
    }

    #[test]
    fn parse_invalid_errors() {
        assert!(parse_value("xyz", "dec").is_err());
        assert!(parse_value("gg", "hex").is_err());
    }

    // --- render ---

    #[test]
    fn render_dec() {
        assert_eq!(render(255, "dec"), "255");
    }

    #[test]
    fn render_hex_uppercase() {
        assert_eq!(render(255, "hex"), "FF");
        assert_eq!(render(10, "hex"), "A");
    }

    #[test]
    fn render_oct() {
        assert_eq!(render(255, "oct"), "377");
    }

    #[test]
    fn render_bin() {
        assert_eq!(render(255, "bin"), "11111111");
    }

    // --- run ---

    #[test]
    fn run_default_all_bases() {
        run("255", &[], "dec", false).unwrap();
    }

    #[test]
    fn run_single_target_bare() {
        // Should not error; bare output is just the value
        run("255", &[String::from("hex")], "dec", false).unwrap();
    }

    #[test]
    fn run_two_targets_labeled() {
        run(
            "255",
            &[String::from("hex"), String::from("bin")],
            "dec",
            false,
        )
        .unwrap();
    }

    #[test]
    fn run_json_output() {
        run("255", &[], "dec", true).unwrap();
    }

    #[test]
    fn run_hex_input_explicit() {
        run("ff", &[], "hex", false).unwrap();
    }

    #[test]
    fn run_auto_detect_0x() {
        run("0xff", &[], "auto", false).unwrap();
    }

    #[test]
    fn run_unknown_from_errors() {
        let r = run("255", &[], "base7", false);
        assert!(matches!(r, Err(crate::error::Error::Usage(_))));
    }

    #[test]
    fn run_unknown_target_errors() {
        let r = run("255", &[String::from("base7")], "dec", false);
        assert!(matches!(r, Err(crate::error::Error::Usage(_))));
    }

    #[test]
    fn run_invalid_input_errors() {
        let r = run("xyz", &[], "dec", false);
        assert!(matches!(r, Err(crate::error::Error::Input(_))));
    }

    #[test]
    fn run_zero() {
        run("0", &[], "dec", false).unwrap();
    }

    #[test]
    fn run_large_number() {
        run("4294967295", &[], "dec", false).unwrap(); // u32::MAX
    }
}
