use crate::error::Error;
use serde_json::{json, Value};

/// Named Unicode ranges available to `--range`.
///
/// Each entry: (name, start_codepoint, end_codepoint).
pub const RANGES: &[(&str, u32, u32)] = &[
    ("arrows", 0x2190, 0x21FF),
    ("latin", 0x0020, 0x007E),
    ("latin-extended", 0x00A0, 0x024F),
    ("greek", 0x0370, 0x03FF),
    ("cyrillic", 0x0400, 0x04FF),
    ("hebrew", 0x0590, 0x05FF),
    ("arabic", 0x0600, 0x06FF),
    ("cjk", 0x4E00, 0x9FFF),
    ("symbols", 0x2600, 0x26FF),
    ("dingbats", 0x2700, 0x27BF),
    ("math", 0x2200, 0x22FF),
    ("emoji", 0x1F600, 0x1F64F),
    ("box-drawing", 0x2500, 0x257F),
];

/// Parse user input as a codepoint or single character.
///
/// Accepts: `U+2603`, `u+2603`, `0x2603`, `2603` (hex when 4+ hex digits), or a single char.
pub fn parse_input(input: &str) -> Result<u32, Error> {
    let s = input.trim();
    if s.is_empty() {
        return Err(Error::Usage("glyph: empty input".into()));
    }

    // U+XXXX or u+XXXX
    if let Some(rest) = s.strip_prefix("U+").or_else(|| s.strip_prefix("u+")) {
        return u32::from_str_radix(rest, 16)
            .map_err(|_| Error::Input(format!("glyph: invalid codepoint '{input}'")));
    }

    // 0xXXXX
    if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u32::from_str_radix(rest, 16)
            .map_err(|_| Error::Input(format!("glyph: invalid codepoint '{input}'")));
    }

    // Single character?
    let chars: Vec<char> = s.chars().collect();
    if chars.len() == 1 {
        return Ok(chars[0] as u32);
    }

    // Bare hex string (be permissive — try parsing as hex).
    if s.chars().all(|c| c.is_ascii_hexdigit()) {
        return u32::from_str_radix(s, 16)
            .map_err(|_| Error::Input(format!("glyph: invalid codepoint '{input}'")));
    }

    Err(Error::Input(format!(
        "glyph: cannot parse '{input}' as codepoint or single character"
    )))
}

/// Format codepoint as `U+XXXX` with at least 4 hex digits.
fn fmt_u(cp: u32) -> String {
    format!("U+{:04X}", cp)
}

/// HTML entity: `&#xXXXX;`.
fn fmt_html(cp: u32) -> String {
    format!("&#x{:X};", cp)
}

/// CSS unicode escape: `\XXXXXX` — pad to 6 digits, as in the web tool.
fn fmt_css(cp: u32) -> String {
    format!("\\{:06X}", cp)
}

/// JS escape: `\uXXXX` for BMP; `\u{XXXXX}` for supplementary.
fn fmt_js(cp: u32) -> String {
    if cp <= 0xFFFF {
        format!("\\u{:04X}", cp)
    } else {
        format!("\\u{{{:X}}}", cp)
    }
}

/// Look up the Unicode character name (e.g. "SNOWMAN"), if any.
fn name_of(ch: char) -> Option<String> {
    unicode_names2::name(ch).map(|n| n.to_string())
}

/// Filter check: does the character's Unicode name contain `query` (case-insensitive)?
fn name_matches(ch: char, query_upper: &str) -> bool {
    match name_of(ch) {
        Some(n) => n.to_ascii_uppercase().contains(query_upper),
        None => false,
    }
}

/// Build a JSON object for a single codepoint.
fn glyph_json(cp: u32) -> Value {
    let ch = char::from_u32(cp);
    json!({
        "codepoint": fmt_u(cp),
        "char": ch.map(|c| c.to_string()),
        "name": ch.and_then(name_of),
        "html": fmt_html(cp),
        "css": fmt_css(cp),
        "js": fmt_js(cp),
    })
}

/// Plain-text single-glyph line, matching the spec example.
fn glyph_line(cp: u32) -> String {
    let ch = char::from_u32(cp);
    let ch_disp = ch.map(|c| c.to_string()).unwrap_or_else(|| "?".into());
    let name = ch.and_then(name_of).unwrap_or_else(|| "(unnamed)".into());
    format!(
        "{}  {}  HTML: {}  CSS: {}  JS: {}  name: {}",
        ch_disp,
        fmt_u(cp),
        fmt_html(cp),
        fmt_css(cp),
        fmt_js(cp),
        name
    )
}

pub fn run(
    input: Option<&str>,
    range: Option<&str>,
    search: Option<&str>,
    limit: usize,
    as_json: bool,
) -> Result<(), Error> {
    // --search: scan ranges (and a generous BMP+supplementary window) for name matches.
    if let Some(query) = search {
        let q = query.trim();
        if q.is_empty() {
            return Err(Error::Usage("glyph: --search query is empty".into()));
        }
        let q_upper = q.to_ascii_uppercase();

        // Walk every codepoint that has a Unicode name.
        // Bounded by the union of all named ranges (assigned BMP + a chunk of SMP).
        let mut found: Vec<u32> = Vec::new();
        // Scan up to U+1FFFF — covers everything in our named ranges plus emoji.
        for cp in 0x0020u32..=0x1FFFFu32 {
            if let Some(ch) = char::from_u32(cp) {
                if name_matches(ch, &q_upper) {
                    found.push(cp);
                    if found.len() >= limit {
                        break;
                    }
                }
            }
        }

        return emit_many(&found, as_json);
    }

    // --range: enumerate a named block.
    if let Some(name) = range {
        let n = name.trim().to_ascii_lowercase();
        let entry = RANGES.iter().find(|(k, _, _)| *k == n).ok_or_else(|| {
            let names: Vec<&str> = RANGES.iter().map(|(k, _, _)| *k).collect();
            Error::Usage(format!(
                "glyph: unknown range '{name}'; valid ranges: {}",
                names.join(", ")
            ))
        })?;
        let (_, start, end) = *entry;

        let mut cps: Vec<u32> = Vec::new();
        for cp in start..=end {
            if char::from_u32(cp).is_some() {
                cps.push(cp);
                if cps.len() >= limit {
                    break;
                }
            }
        }
        return emit_many(&cps, as_json);
    }

    // Positional input: codepoint or single character.
    let input = input.ok_or_else(|| {
        Error::Usage(
            "glyph: provide a codepoint (e.g. U+2603), a character, --range NAME, or --search QUERY"
                .into(),
        )
    })?;
    let cp = parse_input(input)?;

    if as_json {
        println!("{}", serde_json::to_string_pretty(&glyph_json(cp)).unwrap());
    } else {
        println!("{}", glyph_line(cp));
    }
    Ok(())
}

fn emit_many(cps: &[u32], as_json: bool) -> Result<(), Error> {
    if as_json {
        let arr: Vec<Value> = cps.iter().map(|&cp| glyph_json(cp)).collect();
        println!("{}", serde_json::to_string_pretty(&Value::Array(arr)).unwrap());
    } else {
        for &cp in cps {
            println!("{}", glyph_line(cp));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_u_plus_hex() {
        assert_eq!(parse_input("U+2603").unwrap(), 0x2603);
        assert_eq!(parse_input("u+2603").unwrap(), 0x2603);
    }

    #[test]
    fn parse_0x_hex() {
        assert_eq!(parse_input("0x41").unwrap(), 0x41);
    }

    #[test]
    fn parse_single_char() {
        assert_eq!(parse_input("☃").unwrap(), 0x2603);
        assert_eq!(parse_input("A").unwrap(), 0x41);
    }

    #[test]
    fn parse_bare_hex() {
        assert_eq!(parse_input("2603").unwrap(), 0x2603);
    }

    #[test]
    fn parse_empty_errors() {
        assert!(parse_input("").is_err());
    }

    #[test]
    fn parse_multi_char_non_hex_errors() {
        assert!(parse_input("hi there").is_err());
    }

    #[test]
    fn format_codepoint() {
        assert_eq!(fmt_u(0x2603), "U+2603");
        assert_eq!(fmt_u(0x41), "U+0041");
        assert_eq!(fmt_u(0x1F600), "U+1F600");
    }

    #[test]
    fn format_html() {
        assert_eq!(fmt_html(0x2603), "&#x2603;");
    }

    #[test]
    fn format_css() {
        assert_eq!(fmt_css(0x2603), "\\002603");
    }

    #[test]
    fn format_js_bmp() {
        assert_eq!(fmt_js(0x2603), "\\u2603");
    }

    #[test]
    fn format_js_smp() {
        assert_eq!(fmt_js(0x1F600), "\\u{1F600}");
    }

    #[test]
    fn snowman_has_name() {
        assert_eq!(name_of('☃').unwrap(), "SNOWMAN");
    }

    #[test]
    fn ranges_contain_arrows() {
        let arrows = RANGES.iter().find(|(k, _, _)| *k == "arrows").unwrap();
        assert_eq!(arrows.1, 0x2190);
        assert_eq!(arrows.2, 0x21FF);
    }

    #[test]
    fn range_slicing_respects_limit() {
        // Run the function — should not panic.
        run(None, Some("arrows"), None, 5, false).unwrap();
        run(None, Some("arrows"), None, 5, true).unwrap();
    }

    #[test]
    fn unknown_range_errors() {
        let r = run(None, Some("nonsense"), None, 10, false);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn run_codepoint_lookup() {
        run(Some("U+2603"), None, None, 50, false).unwrap();
        run(Some("☃"), None, None, 50, false).unwrap();
    }

    #[test]
    fn run_search() {
        run(None, None, Some("snowman"), 5, false).unwrap();
    }

    #[test]
    fn run_no_args_errors() {
        let r = run(None, None, None, 50, false);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn run_empty_search_errors() {
        let r = run(None, None, Some(""), 50, false);
        assert!(matches!(r, Err(Error::Usage(_))));
    }
}
