use crate::error::Error;
use crate::input;
use regex::RegexBuilder;
use serde_json::{json, Value};

/// Apply a flag string (e.g. "gim") to a RegexBuilder.
/// Returns `find_all` = true if `g` is present (default).
fn apply_flags(builder: &mut RegexBuilder, flags: &str) -> Result<bool, Error> {
    let mut find_all = false;

    for ch in flags.chars() {
        match ch {
            'g' => {
                find_all = true;
            }
            'i' => {
                builder.case_insensitive(true);
            }
            'm' => {
                builder.multi_line(true);
            }
            's' => {
                builder.dot_matches_new_line(true);
            }
            'x' => {
                builder.ignore_whitespace(true);
            }
            // Tolerate but ignore unicode/sticky flags people pass in by habit.
            'u' | 'y' => {}
            other => {
                return Err(Error::Usage(format!(
                    "regex: unknown flag '{other}' (valid: g, i, m, s, x)"
                )));
            }
        }
    }

    Ok(find_all)
}

pub fn run(
    pattern: &str,
    text: Option<&str>,
    flags: &str,
    as_json: bool,
) -> Result<(), Error> {
    if pattern.is_empty() {
        return Err(Error::Usage("regex: empty pattern".into()));
    }

    let mut builder = RegexBuilder::new(pattern);
    let find_all = apply_flags(&mut builder, flags)?;

    let re = builder
        .build()
        .map_err(|e| Error::Input(format!("regex: invalid pattern: {e}")))?;

    let text = input::read_text(text)?;

    // Collect matches.
    let mut matches: Vec<(usize, usize, String, Vec<Option<(usize, usize, String)>>)> =
        Vec::new();

    let captures_iter: Box<dyn Iterator<Item = regex::Captures<'_>>> = if find_all {
        Box::new(re.captures_iter(&text))
    } else {
        Box::new(re.captures(&text).into_iter())
    };

    for cap in captures_iter {
        let m0 = match cap.get(0) {
            Some(m) => m,
            None => continue,
        };
        let mut groups: Vec<Option<(usize, usize, String)>> = Vec::new();
        // cap.len() includes group 0 — skip it.
        for i in 1..cap.len() {
            groups.push(
                cap.get(i)
                    .map(|m| (m.start(), m.end(), m.as_str().to_string())),
            );
        }
        matches.push((m0.start(), m0.end(), m0.as_str().to_string(), groups));
    }

    if as_json {
        let arr: Vec<Value> = matches
            .iter()
            .map(|(start, end, value, groups)| {
                let groups_json: Vec<Value> = groups
                    .iter()
                    .map(|g| match g {
                        Some((s, e, v)) => json!({
                            "value": v,
                            "start": s,
                            "end": e,
                        }),
                        None => Value::Null,
                    })
                    .collect();
                json!({
                    "value": value,
                    "start": start,
                    "end": end,
                    "groups": groups_json,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&Value::Array(arr)).unwrap());
        return Ok(());
    }

    if matches.is_empty() {
        println!("No matches.");
        return Ok(());
    }

    for (i, (start, end, value, groups)) in matches.iter().enumerate() {
        println!("Match {}: \"{}\" (pos {}-{})", i + 1, value, start, end);
        for (gi, g) in groups.iter().enumerate() {
            match g {
                Some((s, e, v)) => {
                    println!("  Group {}: \"{}\" (pos {}-{})", gi + 1, v, s, e);
                }
                None => {
                    println!("  Group {}: <none>", gi + 1);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(pattern: &str, text: &str, flags: &str) -> Vec<(usize, usize, String)> {
        let mut b = RegexBuilder::new(pattern);
        let find_all = apply_flags(&mut b, flags).unwrap();
        let re = b.build().unwrap();
        if find_all {
            re.find_iter(text)
                .map(|m| (m.start(), m.end(), m.as_str().to_string()))
                .collect()
        } else {
            re.find(text)
                .map(|m| vec![(m.start(), m.end(), m.as_str().to_string())])
                .unwrap_or_default()
        }
    }

    #[test]
    fn matches_digits() {
        let r = find(r"\d+", "abc 123 def 456", "g");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].2, "123");
        assert_eq!(r[1].2, "456");
        assert_eq!(r[0].0, 4); // position of "123"
        assert_eq!(r[0].1, 7);
        assert_eq!(r[1].0, 12); // position of "456"
        assert_eq!(r[1].1, 15);
    }

    #[test]
    fn flag_i_case_insensitive() {
        let r = find(r"hello", "HELLO world", "i");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].2, "HELLO");
    }

    #[test]
    fn flag_m_multiline() {
        let r = find(r"^foo", "bar\nfoo", "gm");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn flag_s_dotall() {
        let r = find(r"a.b", "a\nb", "s");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn no_global_returns_first_only() {
        let r = find(r"\d+", "1 2 3", "");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].2, "1");
    }

    #[test]
    fn empty_flags_no_global() {
        // Empty flag string means no 'g', so find_first only.
        let mut b = RegexBuilder::new(r"\d+");
        let global = apply_flags(&mut b, "").unwrap();
        assert!(!global);
    }

    #[test]
    fn g_flag_enables_global() {
        let mut b = RegexBuilder::new(r"\d+");
        let global = apply_flags(&mut b, "g").unwrap();
        assert!(global);
    }

    #[test]
    fn unknown_flag_errors() {
        let mut b = RegexBuilder::new(r".");
        let r = apply_flags(&mut b, "z");
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn capture_groups() {
        let re = RegexBuilder::new(r"(\w+)=(\d+)").build().unwrap();
        let caps = re.captures("a=1").unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "a");
        assert_eq!(caps.get(2).unwrap().as_str(), "1");
    }

    #[test]
    fn run_basic() {
        run(r"\d+", Some("abc 123 def 456"), "g", false).unwrap();
    }

    #[test]
    fn run_json() {
        run(r"\d+", Some("abc 123 def 456"), "g", true).unwrap();
    }

    #[test]
    fn run_no_matches() {
        run(r"zzz", Some("hello"), "g", false).unwrap();
    }

    #[test]
    fn run_invalid_pattern_errors() {
        // Unbalanced paren — regex compile failure.
        let r = run(r"(abc", Some("abc"), "g", false);
        assert!(matches!(r, Err(Error::Input(_))));
    }

    #[test]
    fn run_empty_pattern_errors() {
        let r = run("", Some("abc"), "g", false);
        assert!(matches!(r, Err(Error::Usage(_))));
    }
}
