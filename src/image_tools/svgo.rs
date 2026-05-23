use crate::error::Error;
use crate::image_tools::{derive_output, resolve_output};
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use serde_json::json;
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Optimise an SVG document.
///
/// Passes (all conservative — we never drop anything we can't prove is default):
/// 1. Strip XML/SVG comments.
/// 2. Drop whitespace-only text between elements.
/// 3. Drop default attributes: `version="1.1"`, `x="0"`, `y="0"`,
///    and `xmlns:xlink` declarations when no `xlink:` reference exists.
/// 4. Tidy `style="..."`: collapse whitespace, drop trailing `;`.
/// 5. Round numeric attribute values to 3 decimals (skipping `d`/`points` —
///    they mix command letters with numbers and need careful per-number handling).
/// 6. Re-serialise with no indentation.
pub fn optimise(input: &str) -> Result<String, Error> {
    let uses_xlink = detect_xlink_refs(input);

    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(false);
    reader.config_mut().expand_empty_elements = false;

    let mut writer = Writer::new(Cursor::new(Vec::<u8>::new()));
    let mut buf = Vec::new();
    // Track the open-element stack so we can preserve whitespace inside elements
    // where SVG treats it as significant (text content, descriptions, scripts,
    // foreignObject, or anywhere `xml:space="preserve"` is set).
    let mut stack: Vec<String> = Vec::new();
    let mut preserve_ws = false;

    loop {
        let evt = reader
            .read_event_into(&mut buf)
            .map_err(|e| Error::Processing(format!("svgo: parse error: {e}")))?;

        match evt {
            // strip comments
            Event::Comment(_) => {}

            // drop whitespace-only text between elements, unless we're inside
            // text content where whitespace is significant.
            Event::Text(t) => {
                let raw: &[u8] = t.as_ref();
                let is_ws_only = raw.iter().all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'));
                if is_ws_only && !preserve_ws && !inside_ws_significant(&stack) {
                    // skip — whitespace between tags is not meaningful for layout in SVG
                } else {
                    writer
                        .write_event(Event::Text(t))
                        .map_err(|e| Error::Processing(format!("svgo: write error: {e}")))?;
                }
            }

            Event::Start(e) => {
                let tag = std::str::from_utf8(e.name().as_ref())
                    .map_err(|err| Error::Processing(format!("svgo: tag not utf-8: {err}")))?
                    .to_string();
                if has_xml_space_preserve(&e) {
                    preserve_ws = true;
                }
                stack.push(tag);
                let cleaned = clean_start(&e, uses_xlink)?;
                writer
                    .write_event(Event::Start(cleaned))
                    .map_err(|err| Error::Processing(format!("svgo: write error: {err}")))?;
            }
            Event::End(e) => {
                stack.pop();
                // Recompute preserve_ws by re-walking the stack — cheap enough.
                preserve_ws = false; // any preserve attribute would need to be re-established by a parent
                writer
                    .write_event(Event::End(e))
                    .map_err(|err| Error::Processing(format!("svgo: write error: {err}")))?;
            }
            Event::Empty(e) => {
                let cleaned = clean_start(&e, uses_xlink)?;
                writer
                    .write_event(Event::Empty(cleaned))
                    .map_err(|err| Error::Processing(format!("svgo: write error: {err}")))?;
            }

            Event::Eof => break,
            other => {
                writer
                    .write_event(other)
                    .map_err(|err| Error::Processing(format!("svgo: write error: {err}")))?;
            }
        }

        buf.clear();
    }

    let bytes = writer.into_inner().into_inner();
    String::from_utf8(bytes)
        .map_err(|e| Error::Processing(format!("svgo: produced invalid utf-8: {e}")))
}

/// SVG elements where whitespace is semantically significant — collapsing
/// it changes the rendered output.
fn inside_ws_significant(stack: &[String]) -> bool {
    stack.iter().any(|s| {
        matches!(
            s.as_str(),
            "text" | "tspan" | "textPath" | "tref" | "title" | "desc" | "script" | "style"
        )
    })
}

/// True if a Start event carries `xml:space="preserve"`.
fn has_xml_space_preserve(e: &BytesStart<'_>) -> bool {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"xml:space" {
            if attr.value.as_ref() == b"preserve" {
                return true;
            }
        }
    }
    false
}

/// Heuristic: any occurrence of `xlink:` in the source means we must keep the
/// xlink namespace declaration. Cheap substring scan beats a second parse pass.
fn detect_xlink_refs(svg: &str) -> bool {
    // ignore the namespace declaration itself; look for actual use
    let mut idx = 0;
    while let Some(found) = svg[idx..].find("xlink:") {
        let absolute = idx + found;
        // skip if this is part of `xmlns:xlink="..."` declaration
        let preceding = &svg[..absolute];
        if preceding.trim_end().ends_with("xmlns:") {
            idx = absolute + "xlink:".len();
            continue;
        }
        return true;
    }
    false
}

fn clean_start<'a>(start: &BytesStart<'a>, uses_xlink: bool) -> Result<BytesStart<'static>, Error> {
    let name_bytes = start.name().as_ref().to_vec();
    let name = std::str::from_utf8(&name_bytes)
        .map_err(|e| Error::Processing(format!("svgo: tag name not utf-8: {e}")))?
        .to_string();

    let mut out = BytesStart::new(name.clone());

    for attr in start.attributes() {
        let attr: Attribute = attr
            .map_err(|e| Error::Processing(format!("svgo: bad attribute: {e}")))?;
        let key_bytes = attr.key.as_ref().to_vec();
        let value_bytes = attr.value.as_ref().to_vec();
        let key = std::str::from_utf8(&key_bytes)
            .map_err(|e| Error::Processing(format!("svgo: attr name not utf-8: {e}")))?;
        let value = std::str::from_utf8(&value_bytes)
            .map_err(|e| Error::Processing(format!("svgo: attr value not utf-8: {e}")))?;

        // pass 3: drop attributes that have provably-default values
        if is_default_attr(&name, key, value) {
            continue;
        }
        // drop xlink namespace when there are no xlink: references
        if key == "xmlns:xlink" && !uses_xlink {
            continue;
        }

        // pass 4: tidy style attribute
        let cleaned_value = if key == "style" {
            tidy_style(value)
        } else if should_round_attr(key) {
            // pass 5: round numeric attribute values
            round_numeric(value)
        } else {
            value.to_string()
        };

        // skip empty style after tidying
        if key == "style" && cleaned_value.is_empty() {
            continue;
        }

        // Use the raw-bytes overload so quick-xml does not re-escape values
        // we just read raw from the source (e.g. `xlink:href="a&amp;b"` must not
        // become `a&amp;amp;b`).
        out.push_attribute(quick_xml::events::attributes::Attribute::from((
            key.as_bytes(),
            cleaned_value.as_bytes(),
        )));
    }

    Ok(out)
}

/// True if this attribute carries its default value for `<tag>` and can be dropped.
fn is_default_attr(tag: &str, key: &str, value: &str) -> bool {
    match (tag, key) {
        ("svg", "version") => value == "1.1",
        // x/y default to 0 on positioned elements
        (_, "x") | (_, "y") => value == "0" || value == "0px",
        // many shape attrs default to 0
        ("rect", "rx") | ("rect", "ry") => value == "0",
        ("line" | "rect" | "circle" | "ellipse" | "polygon" | "polyline" | "path", "stroke")
            if value == "none" =>
        {
            true
        }
        (_, "fill-opacity") | (_, "stroke-opacity") | (_, "opacity") => value == "1",
        _ => false,
    }
}

/// Attributes whose value is a single number (or whitespace-separated list of
/// numbers) and can be uniformly rounded. We *deliberately* exclude `d`
/// (path data — contains command letters) and `points` (we keep these for now;
/// rounding them safely needs a per-token approach).
fn should_round_attr(key: &str) -> bool {
    matches!(
        key,
        "x" | "y"
            | "x1"
            | "y1"
            | "x2"
            | "y2"
            | "cx"
            | "cy"
            | "r"
            | "rx"
            | "ry"
            | "width"
            | "height"
            | "stroke-width"
            | "font-size"
    )
}

/// Round any floating-point numbers in `value` to 3 decimals; whitespace
/// preserved.
fn round_numeric(value: &str) -> String {
    let trimmed = value.trim();
    if let Ok(n) = trimmed.parse::<f64>() {
        return format_number(n);
    }
    // multiple numbers separated by whitespace/commas — handle each
    let mut out = String::with_capacity(value.len());
    let mut cur = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
            cur.push(c);
        } else {
            if !cur.is_empty() {
                if let Ok(n) = cur.parse::<f64>() {
                    out.push_str(&format_number(n));
                } else {
                    out.push_str(&cur);
                }
                cur.clear();
            }
            out.push(c);
        }
    }
    if !cur.is_empty() {
        if let Ok(n) = cur.parse::<f64>() {
            out.push_str(&format_number(n));
        } else {
            out.push_str(&cur);
        }
    }
    out
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e16 {
        return format!("{}", n as i64);
    }
    let rounded = (n * 1000.0).round() / 1000.0;
    // Drop trailing zeros: "1.500" → "1.5", "1.000" → "1"
    let s = format!("{rounded:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

/// Collapse whitespace, drop the trailing `;`, and strip empty declarations.
fn tidy_style(value: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for decl in value.split(';') {
        let trimmed = decl.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((prop, val)) = trimmed.split_once(':') {
            parts.push(format!("{}:{}", prop.trim(), val.trim()));
        } else {
            parts.push(trimmed.to_string());
        }
    }
    parts.join(";")
}

pub fn run(
    files: &[PathBuf],
    json_out: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if files.is_empty() {
        return Err(Error::Usage("svgo: no input files".into()));
    }

    let n = files.len();
    let mut results: Vec<serde_json::Value> = Vec::with_capacity(n);
    let mut total_in: u64 = 0;
    let mut total_out: u64 = 0;
    let mut failures = 0;

    for file in files {
        match process_one(file, output, n) {
            Ok((out_path, in_size, out_size)) => {
                total_in += in_size;
                total_out += out_size;
                let reduction = if in_size > 0 {
                    1.0 - (out_size as f64 / in_size as f64)
                } else {
                    0.0
                };

                if json_out {
                    results.push(json!({
                        "result": out_path.display().to_string(),
                        "original_size": in_size,
                        "optimised_size": out_size,
                        "reduction": (reduction * 10000.0).round() / 10000.0,
                    }));
                } else if !quiet {
                    println!(
                        "{} -> {} ({} B -> {} B, {:.1}% reduction)",
                        file.display(),
                        out_path.display(),
                        in_size,
                        out_size,
                        reduction * 100.0,
                    );
                }
            }
            Err(e) => {
                failures += 1;
                if json_out {
                    results.push(json!({
                        "file": file.display().to_string(),
                        "error": format!("{e}"),
                    }));
                } else {
                    eprintln!("error: {}: {e}", file.display());
                }
            }
        }
    }

    if json_out {
        if n == 1 {
            println!("{}", serde_json::to_string_pretty(&results[0]).unwrap());
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "results": results,
                    "total_original": total_in,
                    "total_optimised": total_out,
                }))
                .unwrap()
            );
        }
    } else if !quiet && n > 1 {
        let pct = if total_in > 0 {
            (1.0 - (total_out as f64 / total_in as f64)) * 100.0
        } else {
            0.0
        };
        println!(
            "total: {} B -> {} B ({:.1}% reduction across {} file{})",
            total_in,
            total_out,
            pct,
            n,
            if n == 1 { "" } else { "s" }
        );
    }

    if failures > 0 {
        return Err(Error::Processing(format!(
            "svgo: {failures} of {n} file(s) failed"
        )));
    }
    Ok(())
}

fn process_one(
    file: &Path,
    user_output: Option<&Path>,
    n_inputs: usize,
) -> Result<(PathBuf, u64, u64), Error> {
    let input = std::fs::read_to_string(file)
        .map_err(|e| Error::Input(format!("{}: {e}", file.display())))?;
    let in_size = input.len() as u64;
    let optimised = optimise(&input)?;
    let out_size = optimised.len() as u64;

    let derived = derive_output(file, "optimised", Some("svg"));
    let out_path = resolve_output(user_output, n_inputs, &derived)?;
    std::fs::write(&out_path, &optimised)
        .map_err(|e| Error::Processing(format!("could not write {}: {e}", out_path.display())))?;

    Ok((out_path, in_size, out_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_comments() {
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg"><!-- a comment --><rect/></svg>"#;
        let out = optimise(input).unwrap();
        assert!(!out.contains("comment"));
        assert!(!out.contains("<!--"));
    }

    #[test]
    fn drops_whitespace_only_text() {
        let input = "<svg xmlns=\"http://www.w3.org/2000/svg\">\n  \n  <rect/>\n</svg>";
        let out = optimise(input).unwrap();
        // No raw whitespace between tags
        assert!(!out.contains("\n  \n"));
    }

    #[test]
    fn preserves_meaningful_text() {
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg"><text>hello world</text></svg>"#;
        let out = optimise(input).unwrap();
        assert!(out.contains("hello world"));
    }

    #[test]
    fn drops_default_version() {
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg" version="1.1"><rect/></svg>"#;
        let out = optimise(input).unwrap();
        assert!(!out.contains("version="));
        assert!(out.contains("xmlns"));
    }

    #[test]
    fn drops_default_xy() {
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect x="0" y="0" width="10" height="10"/></svg>"#;
        let out = optimise(input).unwrap();
        assert!(!out.contains(r#"x="0""#));
        assert!(!out.contains(r#"y="0""#));
        assert!(out.contains(r#"width="10""#));
    }

    #[test]
    fn drops_xlink_when_unused() {
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><rect/></svg>"#;
        let out = optimise(input).unwrap();
        assert!(!out.contains("xmlns:xlink"));
    }

    #[test]
    fn keeps_xlink_when_used() {
        let input = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><use xlink:href="#foo"/></svg>"##;
        let out = optimise(input).unwrap();
        assert!(out.contains("xmlns:xlink"));
        assert!(out.contains("xlink:href"));
    }

    #[test]
    fn tidies_style() {
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect style="  fill : red ;  stroke:  blue;  "/></svg>"#;
        let out = optimise(input).unwrap();
        assert!(out.contains("fill:red"));
        assert!(out.contains("stroke:blue"));
        assert!(!out.contains("fill : red"));
        // no trailing semicolons
        assert!(!out.contains(";\""));
    }

    #[test]
    fn rounds_numbers() {
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="10.123456" height="20.000"/></svg>"#;
        let out = optimise(input).unwrap();
        assert!(out.contains(r#"width="10.123""#));
        // trailing zeros stripped
        assert!(out.contains(r#"height="20""#));
    }

    #[test]
    fn preserves_path_data_letters() {
        // d is deliberately not rounded — make sure command letters survive
        let input = r#"<svg xmlns="http://www.w3.org/2000/svg"><path d="M10.5 20.5 L30.5 40.5 Z"/></svg>"#;
        let out = optimise(input).unwrap();
        assert!(out.contains('M'));
        assert!(out.contains('L'));
        assert!(out.contains('Z'));
    }

    #[test]
    fn reduces_typical_svg() {
        // A reasonably wordy SVG with comments, whitespace, default attrs, and
        // verbose styles — should produce a meaningful reduction.
        let input = r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- generated by some tool -->
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" version="1.1" width="100" height="100" viewBox="0 0 100 100">
    <!-- main rectangle -->
    <rect x="0" y="0" width="50.000" height="50.000" style=" fill: #ff0000 ; " />
    <!-- inner circle -->
    <circle cx="50.1234567" cy="50.7654321" r="20" />
</svg>
"#;
        let out = optimise(input).unwrap();
        let reduction = 1.0 - (out.len() as f64 / input.len() as f64);
        assert!(
            reduction > 0.20,
            "expected >20% reduction, got {:.1}% ({} -> {})",
            reduction * 100.0,
            input.len(),
            out.len()
        );
    }

    #[test]
    fn handles_malformed_input_gracefully() {
        let input = "<svg><rect/></svg>"; // missing xmlns is fine — XML wise it's valid
        let out = optimise(input).unwrap();
        assert!(out.contains("rect"));
    }

    #[test]
    fn format_number_strips_trailing_zeros() {
        assert_eq!(format_number(20.000), "20");
        assert_eq!(format_number(20.5), "20.5");
        assert_eq!(format_number(20.123456), "20.123");
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(-1.5), "-1.5");
    }

    #[test]
    fn round_numeric_preserves_separators() {
        // viewBox-style space-separated list
        let out = round_numeric("0 0 100.0000 200.5555");
        assert_eq!(out, "0 0 100 200.556");
    }

    #[test]
    fn tidy_style_drops_empty_decls() {
        assert_eq!(tidy_style(";;  ;"), "");
        assert_eq!(tidy_style("fill:red;;"), "fill:red");
    }
}
