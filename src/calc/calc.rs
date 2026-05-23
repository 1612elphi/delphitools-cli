use crate::error::Error;
use crate::input;
use evalexpr::{ContextWithMutableFunctions, ContextWithMutableVariables, Function, HashMapContext, Value};
use serde_json::json;

// ---------------------------------------------------------------------------
// Expression preprocessing
// ---------------------------------------------------------------------------

/// Rewrite calculator-style syntax into evalexpr-compatible syntax:
///   - `5!`  -> `factorial(5)`
///   - `(...)!` -> `factorial((...))`
///   - drop any unicode operator characters that the user might paste.
fn preprocess(expr: &str) -> String {
    // First, normalise common unicode operators.
    let mut s = expr
        .replace('×', "*")
        .replace('·', "*")
        .replace('÷', "/")
        .replace('−', "-") // U+2212 MINUS SIGN
        .replace('π', "pi");

    // Factorial: walk back from each '!' and wrap the preceding number or
    // parenthesised group. We do this iteratively because nested factorials
    // are uncommon but legal: ((3+2)!)!.
    loop {
        let Some(bang) = s.find('!') else { break };
        // Make sure this isn't `!=` (not equal) — leave that alone.
        if s.as_bytes().get(bang + 1) == Some(&b'=') {
            // Look for another '!' later, otherwise stop. To avoid an
            // infinite loop we manually scan from after this position.
            let rest = &s[bang + 2..];
            match rest.find('!') {
                Some(off) => {
                    let new_bang = bang + 2 + off;
                    if s.as_bytes().get(new_bang + 1) == Some(&b'=') {
                        break; // give up; only `!=` operators left
                    }
                    // process this '!' instead by surgery below
                    let replaced = replace_one_factorial(&s, new_bang);
                    if replaced == s {
                        break;
                    }
                    s = replaced;
                    continue;
                }
                None => break,
            }
        }
        let replaced = replace_one_factorial(&s, bang);
        if replaced == s {
            break;
        }
        s = replaced;
    }

    s
}

/// Replace a single `!` at byte offset `bang` with a `factorial(...)` wrap.
fn replace_one_factorial(s: &str, bang: usize) -> String {
    let bytes = s.as_bytes();
    // Walk backwards over spaces to find the end of the operand.
    let mut end = bang;
    while end > 0 && bytes[end - 1] == b' ' {
        end -= 1;
    }
    if end == 0 {
        return s.to_string();
    }
    let last = bytes[end - 1];
    let mut start = end;
    if last == b')' {
        // Match parens backwards.
        let mut depth = 0i32;
        let mut i = end;
        while i > 0 {
            i -= 1;
            match bytes[i] {
                b')' => depth += 1,
                b'(' => {
                    depth -= 1;
                    if depth == 0 {
                        start = i;
                        // If preceded by an identifier, include it (e.g.
                        // `sin(x)!`).
                        let mut j = start;
                        while j > 0 {
                            let c = bytes[j - 1];
                            if c.is_ascii_alphanumeric() || c == b'_' || c == b':' {
                                j -= 1;
                            } else {
                                break;
                            }
                        }
                        start = j;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            return s.to_string();
        }
    } else if last.is_ascii_digit() || last == b'.' {
        let mut i = end;
        while i > 0 {
            let c = bytes[i - 1];
            if c.is_ascii_digit() || c == b'.' {
                i -= 1;
            } else {
                break;
            }
        }
        start = i;
    } else if last.is_ascii_alphabetic() || last == b'_' {
        let mut i = end;
        while i > 0 {
            let c = bytes[i - 1];
            if c.is_ascii_alphanumeric() || c == b'_' {
                i -= 1;
            } else {
                break;
            }
        }
        start = i;
    } else {
        return s.to_string();
    }

    let operand = &s[start..end];
    let mut result = String::with_capacity(s.len() + 16);
    result.push_str(&s[..start]);
    result.push_str("factorial(");
    result.push_str(operand);
    result.push(')');
    result.push_str(&s[bang + 1..]);
    result
}

// ---------------------------------------------------------------------------
// Context builder
// ---------------------------------------------------------------------------

fn arg_to_float(v: &Value) -> Result<f64, evalexpr::EvalexprError> {
    v.as_number()
}

fn unary<F: Fn(f64) -> f64 + Clone + Send + Sync + 'static>(f: F) -> Function {
    Function::new(move |arg| {
        let x = arg_to_float(arg)?;
        Ok(Value::Float(f(x)))
    })
}

fn factorial_fn() -> Function {
    Function::new(|arg| {
        let n = arg.as_number()?;
        if n < 0.0 || !n.is_finite() {
            return Err(evalexpr::EvalexprError::CustomMessage(
                "factorial: argument must be a non-negative integer".into(),
            ));
        }
        let k = n.round();
        if (n - k).abs() > 1e-9 {
            return Err(evalexpr::EvalexprError::CustomMessage(
                "factorial: argument must be a non-negative integer".into(),
            ));
        }
        // Use float so we don't overflow at 21! and above.
        let mut acc = 1.0f64;
        let mut i = 2.0f64;
        while i <= k {
            acc *= i;
            i += 1.0;
        }
        Ok(Value::Float(acc))
    })
}

fn build_context(deg: bool) -> Result<HashMapContext, evalexpr::EvalexprError> {
    let mut ctx = HashMapContext::new();
    let d2r = std::f64::consts::PI / 180.0;
    let r2d = 180.0 / std::f64::consts::PI;

    // Constants.
    ctx.set_value("pi".into(), Value::Float(std::f64::consts::PI))?;
    ctx.set_value("e".into(), Value::Float(std::f64::consts::E))?;
    ctx.set_value("tau".into(), Value::Float(std::f64::consts::TAU))?;

    // Trig — honour deg/rad mode.
    if deg {
        ctx.set_function("sin".into(), unary(move |x| (x * d2r).sin()))?;
        ctx.set_function("cos".into(), unary(move |x| (x * d2r).cos()))?;
        ctx.set_function("tan".into(), unary(move |x| (x * d2r).tan()))?;
        ctx.set_function("asin".into(), unary(move |x| x.asin() * r2d))?;
        ctx.set_function("acos".into(), unary(move |x| x.acos() * r2d))?;
        ctx.set_function("atan".into(), unary(move |x| x.atan() * r2d))?;
    } else {
        ctx.set_function("sin".into(), unary(f64::sin))?;
        ctx.set_function("cos".into(), unary(f64::cos))?;
        ctx.set_function("tan".into(), unary(f64::tan))?;
        ctx.set_function("asin".into(), unary(f64::asin))?;
        ctx.set_function("acos".into(), unary(f64::acos))?;
        ctx.set_function("atan".into(), unary(f64::atan))?;
    }

    // Hyperbolic.
    ctx.set_function("sinh".into(), unary(f64::sinh))?;
    ctx.set_function("cosh".into(), unary(f64::cosh))?;
    ctx.set_function("tanh".into(), unary(f64::tanh))?;

    // Logs / exponentials.
    // Convention: `log` is base-10, `ln` is natural.
    ctx.set_function("log".into(), unary(f64::log10))?;
    ctx.set_function("ln".into(), unary(f64::ln))?;
    ctx.set_function("log10".into(), unary(f64::log10))?;
    ctx.set_function("log2".into(), unary(f64::log2))?;
    ctx.set_function("exp".into(), unary(f64::exp))?;

    // Power / roots / misc.
    ctx.set_function("sqrt".into(), unary(f64::sqrt))?;
    ctx.set_function("cbrt".into(), unary(f64::cbrt))?;
    ctx.set_function("abs".into(), unary(f64::abs))?;
    ctx.set_function("floor".into(), unary(f64::floor))?;
    ctx.set_function("ceil".into(), unary(f64::ceil))?;
    ctx.set_function("round".into(), unary(f64::round))?;
    ctx.set_function("sign".into(), unary(f64::signum))?;

    // Factorial.
    ctx.set_function("factorial".into(), factorial_fn())?;

    Ok(ctx)
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

fn format_result(v: &Value) -> String {
    match v {
        Value::Float(f) => format_float(*f),
        Value::Int(i) => i.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Tuple(t) => {
            let inner: Vec<String> = t.iter().map(format_result).collect();
            format!("({})", inner.join(", "))
        }
        Value::Empty => String::new(),
    }
}

fn format_float(f: f64) -> String {
    if f.is_nan() {
        return "NaN".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if f == 0.0 {
        return "0".into();
    }
    let abs = f.abs();
    if abs < 1e-4 || abs >= 1e15 {
        return format!("{:.6e}", f);
    }
    // Up to ~10 significant digits, trim trailing zeros.
    let mut s = format!("{:.10}", f);
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

pub fn run(expression: Option<&str>, angles: &str, as_json: bool) -> Result<(), Error> {
    let deg = match angles {
        "deg" => true,
        "rad" => false,
        other => {
            return Err(Error::Usage(format!(
                "unknown angle mode '{other}'; expected 'deg' or 'rad'"
            )))
        }
    };

    let raw = input::read_text(expression)?;
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(Error::Usage("empty expression".into()));
    }

    let prepared = preprocess(raw);

    let ctx = build_context(deg)
        .map_err(|e| Error::Processing(format!("context build: {e}")))?;

    let value = evalexpr::eval_with_context(&prepared, &ctx)
        .map_err(|e| Error::Input(format!("{e}")))?;

    let result_str = format_result(&value);

    if as_json {
        let result_json = match value {
            Value::Float(f) => {
                if f.is_finite() {
                    json!(f)
                } else {
                    json!(result_str)
                }
            }
            Value::Int(i) => json!(i),
            Value::Boolean(b) => json!(b),
            Value::String(ref s) => json!(s),
            _ => json!(result_str),
        };
        let obj = json!({ "expression": raw, "result": result_json });
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
    } else {
        println!("{}", result_str);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_to_string(expr: &str, deg: bool) -> String {
        let prepared = preprocess(expr);
        let ctx = build_context(deg).unwrap();
        let v = evalexpr::eval_with_context(&prepared, &ctx).unwrap();
        format_result(&v)
    }

    fn eval_to_float(expr: &str, deg: bool) -> f64 {
        let prepared = preprocess(expr);
        let ctx = build_context(deg).unwrap();
        let v = evalexpr::eval_with_context(&prepared, &ctx).unwrap();
        v.as_number().unwrap()
    }

    #[test]
    fn calc_two_plus_two() {
        assert_eq!(eval_to_string("2+2", false), "4");
    }

    #[test]
    fn calc_sin_zero() {
        let v = eval_to_float("sin(0)", false);
        assert!(v.abs() < 1e-12);
    }

    #[test]
    fn calc_two_pow_ten() {
        let v = eval_to_float("2^10", false);
        assert!((v - 1024.0).abs() < 1e-9);
    }

    #[test]
    fn calc_pi_plus_two_pow_ten() {
        let v = eval_to_float("sin(pi/4) + 2^10", false);
        // sin(pi/4) ≈ 0.7071067811865475
        assert!((v - 1024.7071067811865).abs() < 1e-6);
    }

    #[test]
    fn calc_deg_mode_sin_90() {
        let v = eval_to_float("sin(90)", true);
        assert!((v - 1.0).abs() < 1e-12);
    }

    #[test]
    fn calc_rad_mode_sin_pi_over_2() {
        let v = eval_to_float("sin(pi/2)", false);
        assert!((v - 1.0).abs() < 1e-12);
    }

    #[test]
    fn calc_factorial_basic() {
        let v = eval_to_float("5!", false);
        assert!((v - 120.0).abs() < 1e-9);
    }

    #[test]
    fn calc_factorial_zero() {
        let v = eval_to_float("0!", false);
        assert!((v - 1.0).abs() < 1e-9);
    }

    #[test]
    fn calc_factorial_paren_group() {
        // (2+3)! = 5! = 120
        let v = eval_to_float("(2+3)!", false);
        assert!((v - 120.0).abs() < 1e-9);
    }

    #[test]
    fn calc_multiple_factorials() {
        // 3! + 4! = 6 + 24 = 30
        let v = eval_to_float("3! + 4!", false);
        assert!((v - 30.0).abs() < 1e-9);
    }

    #[test]
    fn calc_log10() {
        let v = eval_to_float("log(100)", false);
        assert!((v - 2.0).abs() < 1e-9);
    }

    #[test]
    fn calc_ln_e() {
        let v = eval_to_float("ln(e)", false);
        assert!((v - 1.0).abs() < 1e-9);
    }

    #[test]
    fn calc_sqrt() {
        let v = eval_to_float("sqrt(16)", false);
        assert!((v - 4.0).abs() < 1e-9);
    }

    #[test]
    fn calc_abs() {
        let v = eval_to_float("abs(-7)", false);
        assert!((v - 7.0).abs() < 1e-9);
    }

    #[test]
    fn calc_exp() {
        let v = eval_to_float("exp(1)", false);
        assert!((v - std::f64::consts::E).abs() < 1e-9);
    }

    #[test]
    fn calc_run_smoke() {
        run(Some("2+2"), "rad", false).unwrap();
    }

    #[test]
    fn calc_run_deg() {
        run(Some("sin(90)"), "deg", false).unwrap();
    }

    #[test]
    fn calc_run_json() {
        run(Some("2+2"), "rad", true).unwrap();
    }

    #[test]
    fn calc_run_bad_angles() {
        let r = run(Some("2+2"), "gradians", false);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn calc_run_parse_error() {
        let r = run(Some("2 +"), "rad", false);
        assert!(matches!(r, Err(Error::Input(_))));
    }

    #[test]
    fn preprocess_replaces_unicode() {
        assert_eq!(preprocess("3 × 4"), "3 * 4");
        assert_eq!(preprocess("3 ÷ 4"), "3 / 4");
        assert_eq!(preprocess("π"), "pi");
    }

    #[test]
    fn preprocess_factorial_number() {
        assert_eq!(preprocess("5!"), "factorial(5)");
        assert_eq!(preprocess("5! + 1"), "factorial(5) + 1");
    }

    #[test]
    fn preprocess_factorial_paren() {
        assert_eq!(preprocess("(2+3)!"), "factorial((2+3))");
    }

    #[test]
    fn preprocess_leaves_not_equal_alone() {
        assert_eq!(preprocess("1 != 2"), "1 != 2");
    }
}
