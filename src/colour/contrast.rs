use crate::colour::Colour;
use crate::error::Error;
use crate::output;
use serde_json::json;

pub fn run(fg_input: &str, bg_input: &str, as_json: bool) -> Result<(), Error> {
    let fg = Colour::parse(fg_input)?;
    let bg = Colour::parse(bg_input)?;

    let l1 = fg.relative_luminance();
    let l2 = bg.relative_luminance();
    let ratio = (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05);

    let aa_normal = ratio >= 4.5;
    let aa_large = ratio >= 3.0;
    let aaa_normal = ratio >= 7.0;
    let aaa_large = ratio >= 4.5;

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ratio": (ratio * 100.0).round() / 100.0,
                "aa_normal": aa_normal,
                "aa_large": aa_large,
                "aaa_normal": aaa_normal,
                "aaa_large": aaa_large,
                "fg": fg.to_hex(),
                "bg": bg.to_hex(),
            }))
            .unwrap()
        );
    } else {
        let (fr, fg_g, fb) = fg.to_u8();
        let (br, bg_g, bb) = bg.to_u8();
        let fg_swatch = output::colour_block(fr, fg_g, fb);
        let bg_swatch = output::colour_block(br, bg_g, bb);

        if !fg_swatch.is_empty() {
            println!("{} {} on {} {}", fg_swatch, fg.to_hex(), bg_swatch, bg.to_hex());
        }

        println!("Ratio: {:.2}:1", ratio);
        println!(
            "AA  normal: {}  large: {}",
            pass_fail(aa_normal),
            pass_fail(aa_large)
        );
        println!(
            "AAA normal: {}  large: {}",
            pass_fail(aaa_normal),
            pass_fail(aaa_large)
        );
    }
    Ok(())
}

fn pass_fail(ok: bool) -> &'static str {
    if ok { "PASS" } else { "FAIL" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast_ratio(fg: &str, bg: &str) -> f64 {
        let fg = Colour::parse(fg).unwrap();
        let bg = Colour::parse(bg).unwrap();
        let l1 = fg.relative_luminance();
        let l2 = bg.relative_luminance();
        (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
    }

    #[test]
    fn black_on_white() {
        let ratio = contrast_ratio("#000000", "#ffffff");
        assert!((ratio - 21.0).abs() < 0.1);
    }

    #[test]
    fn same_colour() {
        let ratio = contrast_ratio("#ff6600", "#ff6600");
        assert!((ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn white_on_black_symmetric() {
        let r1 = contrast_ratio("#000", "#fff");
        let r2 = contrast_ratio("#fff", "#000");
        assert!((r1 - r2).abs() < 0.01);
    }

    #[test]
    fn mid_grey_on_white() {
        // #777 on #fff should be around 4.48:1
        let ratio = contrast_ratio("#777777", "#ffffff");
        assert!(ratio > 4.0 && ratio < 5.0);
    }
}
