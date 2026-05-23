use crate::error::Error;
use serde_json::json;

pub fn run_px2rem(value: f64, base: f64, as_json: bool) -> Result<(), Error> {
    let rem = value / base;
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "px": value,
                "rem": rem,
                "base": base,
            }))
            .unwrap()
        );
    } else {
        println!("{value}px = {rem:.4}rem (base {base}px)");
    }
    Ok(())
}

pub fn run_rem2px(value: f64, base: f64, as_json: bool) -> Result<(), Error> {
    let px = value * base;
    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "rem": value,
                "px": px,
                "base": base,
            }))
            .unwrap()
        );
    } else {
        println!("{value}rem = {px:.1}px (base {base}px)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn px_to_rem_default_base() {
        assert!((16.0_f64 / 16.0 - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn px_to_rem_24() {
        assert!((24.0_f64 / 16.0 - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn rem_to_px() {
        assert!((1.5_f64 * 16.0 - 24.0).abs() < f64::EPSILON);
    }

    #[test]
    fn custom_base() {
        assert!((20.0_f64 / 20.0 - 1.0).abs() < f64::EPSILON);
        assert!((1.5_f64 * 20.0 - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn run_functions_dont_panic() {
        run_px2rem(16.0, 16.0, true).unwrap();
        run_px2rem(24.0, 16.0, false).unwrap();
        run_rem2px(1.5, 16.0, true).unwrap();
        run_rem2px(1.0, 16.0, false).unwrap();
    }
}
