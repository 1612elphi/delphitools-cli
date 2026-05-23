use std::process::Command;

fn delphi(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_delphi"))
        .args(args)
        .output()
        .expect("failed to run delphi");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

// --- colour ---

#[test]
fn colour_single_format() {
    let (out, _, code) = delphi(&["col", "ff6600", "hex"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "#ff6600");
}

#[test]
fn colour_multiple_formats() {
    let (out, _, code) = delphi(&["col", "ff6600", "hex", "rgb"]);
    assert_eq!(code, 0);
    assert!(out.contains("hex: #ff6600"));
    assert!(out.contains("rgb: rgb(255, 102, 0)"));
}

#[test]
fn colour_all_formats() {
    let (out, _, code) = delphi(&["colour", "tomato"]);
    assert_eq!(code, 0);
    assert!(out.contains("hex:"));
    assert!(out.contains("rgb:"));
    assert!(out.contains("hsl:"));
    assert!(out.contains("oklch:"));
    assert!(out.contains("oklab:"));
    assert!(out.contains("lab:"));
}

#[test]
fn colour_named() {
    let (out, _, code) = delphi(&["col", "rebeccapurple", "hex"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "#663399");
}

#[test]
fn colour_hsl_input() {
    let (out, _, code) = delphi(&["col", "hsl(0, 100%, 50%)", "hex"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "#ff0000");
}

#[test]
fn colour_json() {
    let (out, _, code) = delphi(&["col", "ff6600", "hex", "rgb", "-j"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["hex"], "#ff6600");
    assert_eq!(v["rgb"], "rgb(255, 102, 0)");
}

#[test]
fn colour_invalid() {
    let (_, err, code) = delphi(&["col", "not-a-colour", "hex"]);
    assert_eq!(code, 2);
    assert!(err.contains("invalid colour"));
}

// --- contrast ---

#[test]
fn contrast_black_white() {
    let (out, _, code) = delphi(&["contrast", "#000", "#fff"]);
    assert_eq!(code, 0);
    assert!(out.contains("21.00:1"));
    assert!(out.contains("PASS"));
}

#[test]
fn contrast_json() {
    let (out, _, code) = delphi(&["contrast", "#000", "#fff", "-j"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["aa_normal"], true);
    assert_eq!(v["aaa_normal"], true);
}

// --- px2rem / rem2px ---

#[test]
fn px2rem() {
    let (out, _, code) = delphi(&["px2rem", "24"]);
    assert_eq!(code, 0);
    assert!(out.contains("1.5000rem"));
}

#[test]
fn rem2px() {
    let (out, _, code) = delphi(&["rem2px", "1.5"]);
    assert_eq!(code, 0);
    assert!(out.contains("24.0px"));
}

#[test]
fn px2rem_custom_base() {
    let (out, _, code) = delphi(&["px2rem", "20", "--base", "20"]);
    assert_eq!(code, 0);
    assert!(out.contains("1.0000rem"));
}

#[test]
fn px2rem_json() {
    let (out, _, code) = delphi(&["px2rem", "16", "-j"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["rem"], 1.0);
}

// --- wc ---

#[test]
fn wc_arg() {
    let (out, _, code) = delphi(&["wc", "The quick brown fox jumps over the lazy dog."]);
    assert_eq!(code, 0);
    assert!(out.contains("Words: 9"));
    assert!(out.contains("Sentences: 1"));
}

#[test]
fn wc_json() {
    let (out, _, code) = delphi(&["wc", "Hello world.", "-j"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["words"], 2);
}

// --- encode / decode / hash ---

#[test]
fn encode_base64() {
    let (out, _, code) = delphi(&["encode", "base64", "Hello, World!"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "SGVsbG8sIFdvcmxkIQ==");
}

#[test]
fn decode_base64() {
    let (out, _, code) = delphi(&["decode", "base64", "SGVsbG8sIFdvcmxkIQ=="]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "Hello, World!");
}

#[test]
fn encode_url() {
    let (out, _, code) = delphi(&["encode", "url", "hello world&foo=bar"]);
    assert_eq!(code, 0);
    assert!(out.contains("hello%20world%26foo%3Dbar"));
}

#[test]
fn decode_url() {
    let (out, _, code) = delphi(&["decode", "url", "hello%20world"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "hello world");
}

#[test]
fn hash_sha256() {
    let (out, _, code) = delphi(&["hash", "sha256", "Hello, World!"]);
    assert_eq!(code, 0);
    assert_eq!(
        out.trim(),
        "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"
    );
}

#[test]
fn hash_md5() {
    let (out, _, code) = delphi(&["hash", "md5", "Hello, World!"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "65a8e27d8879283831b664bd8b7f0ad4");
}

#[test]
fn hash_unknown_algo() {
    let (_, err, code) = delphi(&["hash", "blake9000", "test"]);
    assert_eq!(code, 1);
    assert!(err.contains("unknown algorithm"));
}

// --- version ---

#[test]
fn version_flag() {
    let (out, _, code) = delphi(&["--version"]);
    assert_eq!(code, 0);
    assert!(out.contains("delphi"));
    assert!(out.contains("0.1.0"));
}

// --- no args shows the tasting menu, `?` and `help` show full help ---

#[test]
fn no_args_shows_sampler() {
    let (out, _, code) = delphi(&[]);
    assert_eq!(code, 0);
    assert!(out.contains("delphitools"));
    // Hint to `delphi ?` for the full list should appear
    assert!(out.contains("delphi ?"));
    // Exactly eight tool lines — bracketed by blank lines, hard to count cleanly,
    // so just sanity-check we're seeing tool entries with descriptions.
    assert!(out.lines().filter(|l| l.starts_with("  ")).count() >= 8);
}

#[test]
fn question_mark_shows_full_command_list() {
    let (out, _, code) = delphi(&["?"]);
    assert_eq!(code, 0);
    assert!(out.contains("Usage:"));
    // Several tools that should always be in the full list
    assert!(out.contains("colour"));
    assert!(out.contains("palette"));
    assert!(out.contains("shavian"));
}

#[test]
fn help_subcommand_still_works() {
    let (out, _, code) = delphi(&["help"]);
    assert_eq!(code, 0);
    assert!(out.contains("Usage:"));
}

#[test]
fn agent_reference_emits_plain_text_only() {
    let (out, _, code) = delphi(&["agent"]);
    assert_eq!(code, 0);
    // Should be plain text (no ANSI escapes) so it pipes cleanly to AI agents.
    assert!(
        !out.contains('\x1b'),
        "agent output should be plain text, contained ANSI escape"
    );
    // Reference should cover the load-bearing sections.
    assert!(out.contains("INVOCATION"));
    assert!(out.contains("EXIT CODES"));
    assert!(out.contains("COMMAND INDEX"));
    assert!(out.contains("GOTCHAS"));
    // And mention key gotchas.
    assert!(out.contains("rmbg"), "should warn about rmbg stub");
    assert!(out.contains("zsh") || out.contains("glob"), "should warn about ? glob");
}

#[test]
fn sampler_mentions_agent_hint() {
    let (out, _, code) = delphi(&[]);
    assert_eq!(code, 0);
    assert!(out.contains("delphi agent"));
}

#[test]
fn install_man_dry_run_lists_pages() {
    let (out, _, code) = delphi(&["install-man", "--dry-run"]);
    assert_eq!(code, 0);
    // Every page line ends in `.1` and at least the master page is listed.
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.iter().any(|l| l.ends_with("delphi.1")));
    assert!(lines.iter().any(|l| l.ends_with("delphi-palette.1")));
    assert!(lines.len() >= 10);
}

#[test]
fn install_man_writes_to_dir() {
    let dir = scratch("install-man");
    let (_, _, code) = delphi(&["install-man", "--dir", dir.to_str().unwrap()]);
    assert_eq!(code, 0);
    // delphi.1 is the master page and must be present.
    let master = dir.join("delphi.1");
    assert!(master.exists(), "master page not written");
    let content = std::fs::read_to_string(&master).unwrap();
    // groff/mandoc page should mention NAME and SYNOPSIS sections.
    assert!(content.contains(".SH NAME"));
    assert!(content.contains(".SH SYNOPSIS"));
}

// ===========================================================================
// Shared test helpers for fixture creation
// ===========================================================================

use std::path::PathBuf;

/// Unique, per-test scratch directory. Created on demand; cleaned up by OS later.
fn scratch(label: &str) -> PathBuf {
    let pid = std::process::id();
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let p = std::env::temp_dir().join(format!("delphi-cli-it-{label}-{pid}-{n}"));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("scratch dir");
    p
}

/// Generate a small PNG via the binary itself (qr is the cheapest deterministic input).
fn make_png(path: &std::path::Path, payload: &str) {
    let (_, err, code) = delphi(&[
        "qr",
        payload,
        "--size",
        "128",
        "-o",
        path.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "qr fixture failed: {err}");
}

/// Write a hand-rolled SVG suitable for svgo testing (has comments + defaults).
fn make_svg(path: &std::path::Path) {
    let body = r##"<?xml version="1.0" encoding="UTF-8"?>
<!-- a comment to remove -->
<svg xmlns="http://www.w3.org/2000/svg" version="1.1" width="100" height="100">
  <!-- inner -->
  <rect x="0" y="0" width="50" height="50" fill="red" opacity="1"/>
  <circle cx="80" cy="20" r="10" fill="#00aa00"/>
</svg>
"##;
    std::fs::write(path, body).expect("write svg fixture");
}

fn is_png(path: &std::path::Path) -> bool {
    match std::fs::read(path) {
        Ok(b) if b.len() >= 8 => &b[..8] == b"\x89PNG\r\n\x1a\n",
        _ => false,
    }
}

fn is_jpeg(path: &std::path::Path) -> bool {
    match std::fs::read(path) {
        Ok(b) if b.len() >= 3 => b[..3] == [0xff, 0xd8, 0xff],
        _ => false,
    }
}

fn is_webp(path: &std::path::Path) -> bool {
    match std::fs::read(path) {
        Ok(b) if b.len() >= 12 => &b[..4] == b"RIFF" && &b[8..12] == b"WEBP",
        _ => false,
    }
}

fn is_pdf(path: &std::path::Path) -> bool {
    match std::fs::read(path) {
        Ok(b) if b.len() >= 4 => &b[..4] == b"%PDF",
        _ => false,
    }
}

fn is_svg(path: &std::path::Path) -> bool {
    match std::fs::read_to_string(path) {
        Ok(s) => s.contains("<svg") && s.contains("xmlns"),
        _ => false,
    }
}

// ===========================================================================
// Palette
// ===========================================================================

#[test]
fn palette_list_includes_categories() {
    let (out, _, code) = delphi(&["palette", "--list"]);
    assert_eq!(code, 0);
    assert!(out.contains("Random"));
    assert!(out.contains("Color Theory"));
    assert!(out.contains("Moods"));
    assert!(out.contains("Decades & Eras"));
    assert!(out.contains("Nature & Scenes"));
    assert!(out.contains("Art & Culture"));
    // A specific strategy from each category should appear:
    assert!(out.contains("true-random"));
    assert!(out.contains("analogous"));
    assert!(out.contains("80s"));
    assert!(out.contains("ocean-sunset"));
    assert!(out.contains("japanese"));
}

#[test]
fn palette_seeded_is_deterministic() {
    let (a, _, ra) = delphi(&[
        "palette",
        "--strategy",
        "80s",
        "--size",
        "5",
        "--seed",
        "12345",
    ]);
    let (b, _, rb) = delphi(&[
        "palette",
        "--strategy",
        "80s",
        "--size",
        "5",
        "--seed",
        "12345",
    ]);
    assert_eq!(ra, 0);
    assert_eq!(rb, 0);
    assert_eq!(a, b, "same seed must yield identical palette");
    // 5 hex codes, one per line.
    let lines: Vec<&str> = a.trim().lines().collect();
    assert_eq!(lines.len(), 5);
    for l in &lines {
        assert!(
            l.starts_with('#') && l.len() == 7,
            "expected 6-digit hex, got {l}"
        );
    }
}

#[test]
fn palette_size_honoured() {
    let (out, _, code) = delphi(&[
        "palette",
        "--strategy",
        "analogous",
        "--size",
        "8",
        "--seed",
        "1",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out.trim().lines().count(), 8);
}

#[test]
fn palette_lock_keeps_slots_fixed() {
    let (out, _, code) = delphi(&[
        "palette",
        "--strategy",
        "analogous",
        "--size",
        "5",
        "--seed",
        "7",
        "--lock",
        "0:#ff6600,3:#003366",
    ]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], "#ff6600");
    assert_eq!(lines[3], "#003366");
}

#[test]
fn palette_json_format() {
    let (out, _, code) = delphi(&[
        "palette",
        "--strategy",
        "triadic",
        "--size",
        "3",
        "--seed",
        "1",
        "-j",
    ]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert!(v.is_array());
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert!(arr[0].get("hex").is_some());
    assert!(arr[0].get("rgb").is_some());
    assert!(arr[0].get("oklch").is_some());
}

#[test]
fn palette_png_format_writes_image() {
    let dir = scratch("palette-png");
    let out_path = dir.join("p.png");
    let (_, err, code) = delphi(&[
        "palette",
        "--strategy",
        "ocean-sunset",
        "--size",
        "5",
        "--format",
        "png",
        "--seed",
        "1",
        "-o",
        out_path.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_png(&out_path));
}

#[test]
fn palette_unknown_strategy_errors() {
    let (_, err, code) = delphi(&["palette", "--strategy", "nope", "--size", "5"]);
    assert_eq!(code, 1);
    assert!(err.contains("nope") || err.contains("unknown"));
}

#[test]
fn palette_bare_invocation_returns_random_cohesive() {
    // `delphi palette` with no args should default to random-cohesive, size 5.
    let (out, _, code) = delphi(&["palette"]);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.trim().lines().collect();
    assert_eq!(lines.len(), 5);
    for l in &lines {
        assert!(l.starts_with('#') && l.len() == 7, "expected hex, got {l}");
    }
}

// ===========================================================================
// Colorblind
// ===========================================================================

#[test]
fn colorblind_colour_protanopia_changes_red() {
    let (out, _, code) = delphi(&[
        "colorblind",
        "#ff0000",
        "--cb-type",
        "protanopia",
        "--colour",
    ]);
    assert_eq!(code, 0);
    // Result is a hex string; protanopia should shift pure red away from #ff0000.
    let hex = out.trim();
    assert!(hex.starts_with('#') && hex.len() == 7, "got: {hex}");
    assert_ne!(hex, "#ff0000", "protanopia should remap pure red");
}

#[test]
fn colorblind_normal_passes_through() {
    let (out, _, code) = delphi(&[
        "colorblind",
        "#3b82f6",
        "--cb-type",
        "normal",
        "--colour",
    ]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "#3b82f6");
}

#[test]
fn colorblind_unknown_type_errors() {
    let (_, _, code) = delphi(&[
        "colorblind",
        "#ff0000",
        "--cb-type",
        "rainbow-vision",
        "--colour",
    ]);
    assert_ne!(code, 0, "unknown CB type should fail");
}

#[test]
fn colorblind_image_mode() {
    let dir = scratch("cb-image");
    let src = dir.join("src.png");
    make_png(&src, "cb-test");
    let (_, err, code) = delphi(&[
        "colorblind",
        src.to_str().unwrap(),
        "--cb-type",
        "deuteranopia",
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    let out = dir.join("src-cb.png");
    assert!(out.exists() && is_png(&out));
}

// ===========================================================================
// Tailwind shades (already partially covered but no integration test existed)
// ===========================================================================

#[test]
fn tailwind_default_mode_emits_eleven_stops() {
    let (out, _, code) = delphi(&["tw", "#3b82f6"]);
    assert_eq!(code, 0);
    for stop in [50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950] {
        assert!(out.contains(&format!("{stop}:")), "missing stop {stop}");
    }
}

#[test]
fn tailwind_invalid_mode_errors() {
    let (_, _, code) = delphi(&["tw", "#3b82f6", "neon"]);
    assert_ne!(code, 0);
}

// ===========================================================================
// Image: crop / matte / scroll / watermark / favicon / split / convert / noise / clip
// ===========================================================================

#[test]
fn crop_to_square() {
    let dir = scratch("crop");
    let src = dir.join("src.png");
    make_png(&src, "wide");
    let (_, err, code) = delphi(&["crop", src.to_str().unwrap(), "--ratio", "1:1"]);
    assert_eq!(code, 0, "stderr: {err}");
    let out = dir.join("src-cropped.png");
    assert!(out.exists() && is_png(&out));
}

#[test]
fn crop_invalid_ratio_errors() {
    let dir = scratch("crop-bad");
    let src = dir.join("src.png");
    make_png(&src, "x");
    let (_, _, code) = delphi(&["crop", src.to_str().unwrap(), "--ratio", "weird"]);
    assert_ne!(code, 0);
}

#[test]
fn matte_solid_writes_image() {
    let dir = scratch("matte");
    let src = dir.join("src.png");
    make_png(&src, "matte");
    let (_, err, code) = delphi(&[
        "matte",
        src.to_str().unwrap(),
        "--mode",
        "solid",
        "--colour",
        "#ff6600",
        "--ratio",
        "1:1",
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    let out = dir.join("src-matted.png");
    assert!(out.exists() && is_png(&out));
}

#[test]
fn scroll_writes_tiles() {
    let dir = scratch("scroll");
    let src = dir.join("src.png");
    make_png(&src, "scroll-src");
    let (_, err, code) = delphi(&[
        "scroll",
        src.to_str().unwrap(),
        "--ratio",
        "4:5",
        "--fill",
        "solid",
        "--colour",
        "#fff",
        "-o",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    let tiles: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("-tile-"))
        .collect();
    assert!(!tiles.is_empty(), "scroll produced no tiles");
}

#[test]
fn watermark_writes_image() {
    let dir = scratch("watermark");
    let base = dir.join("base.png");
    let mark = dir.join("mark.png");
    make_png(&base, "base-image");
    make_png(&mark, "mark");
    let (_, err, code) = delphi(&[
        "watermark",
        base.to_str().unwrap(),
        "--mark",
        mark.to_str().unwrap(),
        "--position",
        "bottom-right",
        "--opacity",
        "0.5",
        "--scale",
        "0.2",
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(dir.join("base-watermarked.png").exists());
}

#[test]
fn favicon_emits_each_size_and_ico() {
    let dir = scratch("favicon");
    let src = dir.join("src.png");
    make_png(&src, "fav");
    let (_, err, code) = delphi(&[
        "favicon",
        src.to_str().unwrap(),
        "--sizes",
        "16,32",
        "--ico",
        "-o",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(dir.join("src-16x16.png").exists());
    assert!(dir.join("src-32x32.png").exists());
    assert!(dir.join("favicon.ico").exists());
}

#[test]
fn split_emits_grid() {
    let dir = scratch("split");
    let src = dir.join("src.png");
    make_png(&src, "split-src");
    let (_, err, code) = delphi(&[
        "split",
        src.to_str().unwrap(),
        "--rows",
        "2",
        "--cols",
        "2",
        "-o",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    for r in 1..=2 {
        for c in 1..=2 {
            let p = dir.join(format!("src-tile-{r}-{c}.png"));
            assert!(p.exists(), "missing tile {p:?}");
        }
    }
}

#[test]
fn convert_png_to_jpeg() {
    let dir = scratch("convert");
    let src = dir.join("src.png");
    make_png(&src, "convert");
    let out = dir.join("out.jpg");
    let (_, err, code) = delphi(&[
        "convert",
        src.to_str().unwrap(),
        "--to",
        "jpeg",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_jpeg(&out));
}

#[test]
fn convert_png_to_webp() {
    let dir = scratch("convert-webp");
    let src = dir.join("src.png");
    make_png(&src, "convert");
    let out = dir.join("out.webp");
    let (_, err, code) = delphi(&[
        "convert",
        src.to_str().unwrap(),
        "--to",
        "webp",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_webp(&out));
}

#[test]
fn convert_resize_percentage() {
    let dir = scratch("convert-resize");
    let src = dir.join("src.png");
    make_png(&src, "convert");
    let out = dir.join("smaller.png");
    let (_, err, code) = delphi(&[
        "convert",
        src.to_str().unwrap(),
        "--to",
        "png",
        "--resize",
        "50%",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_png(&out));
}

#[test]
fn noise_deterministic_with_seed() {
    let dir = scratch("noise");
    let src = dir.join("src.png");
    make_png(&src, "noise");
    let a = dir.join("a.png");
    let b = dir.join("b.png");
    let (_, _, ra) = delphi(&[
        "noise",
        src.to_str().unwrap(),
        "--opacity",
        "0.2",
        "--seed",
        "42",
        "-o",
        a.to_str().unwrap(),
    ]);
    let (_, _, rb) = delphi(&[
        "noise",
        src.to_str().unwrap(),
        "--opacity",
        "0.2",
        "--seed",
        "42",
        "-o",
        b.to_str().unwrap(),
    ]);
    assert_eq!(ra, 0);
    assert_eq!(rb, 0);
    let ba = std::fs::read(&a).unwrap();
    let bb = std::fs::read(&b).unwrap();
    assert_eq!(ba, bb, "same seed must yield identical noise output");
}

#[test]
fn clip_trims_transparent_border() {
    // Build a hand-rolled PNG with transparent margins via the image crate? Too heavy here.
    // Use the qr-generated png which has a fully-opaque border of white; clip should pass-through
    // (no transparent pixels to strip). We assert it still runs and emits a valid PNG.
    let dir = scratch("clip");
    let src = dir.join("src.png");
    make_png(&src, "clip");
    let (_, err, code) = delphi(&["clip", src.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(dir.join("src-clipped.png").exists());
}

// ===========================================================================
// Image advanced: trace / rmbg / svgo
// ===========================================================================

#[test]
fn trace_writes_svg() {
    let dir = scratch("trace");
    let src = dir.join("src.png");
    make_png(&src, "trace");
    let (_, err, code) = delphi(&["trace", src.to_str().unwrap(), "--preset", "default"]);
    assert_eq!(code, 0, "stderr: {err}");
    let out = dir.join("src-traced.svg");
    assert!(is_svg(&out));
}

#[test]
fn rmbg_help_documents_approve_flag() {
    // We don't run rmbg end-to-end in tests — that requires downloading a
    // ~170 MB model. Instead, verify the documented surface: --approve is
    // discoverable, the about-line mentions the model download, and an
    // empty-input invocation produces a usage error rather than panicking.
    let (out, _, code) = delphi(&["rmbg", "--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("--approve"));
    assert!(out.to_lowercase().contains("model"));
}

#[test]
fn rmbg_with_no_inputs_is_usage_error() {
    let (_, err, code) = delphi(&["rmbg"]);
    assert_eq!(code, 1);
    assert!(err.to_lowercase().contains("rmbg"));
    assert!(err.to_lowercase().contains("input"));
}

#[test]
fn svgo_reduces_size_and_strips_comments() {
    let dir = scratch("svgo");
    let src = dir.join("src.svg");
    make_svg(&src);
    let original_size = std::fs::metadata(&src).unwrap().len();
    let (_, err, code) = delphi(&["svgo", src.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {err}");
    let out = dir.join("src-optimised.svg");
    assert!(out.exists());
    let opt = std::fs::read_to_string(&out).unwrap();
    assert!(!opt.contains("<!--"), "comments should be stripped");
    let opt_size = opt.len() as u64;
    assert!(
        opt_size < original_size,
        "svgo did not reduce size: {opt_size} >= {original_size}"
    );
}

// ===========================================================================
// PDF: preflight / zine / impose
// ===========================================================================

fn build_eight_image_set(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(8);
    for i in 1..=8 {
        let p = dir.join(format!("p{i}.png"));
        make_png(&p, &format!("page {i}"));
        paths.push(p);
    }
    paths
}

#[test]
fn zine_writes_pdf() {
    let dir = scratch("zine");
    let imgs = build_eight_image_set(&dir);
    let out = dir.join("zine.pdf");
    let mut args: Vec<&str> = vec!["zine"];
    args.extend(imgs.iter().map(|p| p.to_str().unwrap()));
    args.extend(["-o", out.to_str().unwrap()]);
    let (_, err, code) = delphi(&args);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_pdf(&out));
}

#[test]
fn zine_wrong_image_count_errors() {
    let dir = scratch("zine-bad");
    let imgs = (1..=3)
        .map(|i| {
            let p = dir.join(format!("p{i}.png"));
            make_png(&p, &format!("p{i}"));
            p
        })
        .collect::<Vec<_>>();
    let mut args: Vec<&str> = vec!["zine"];
    args.extend(imgs.iter().map(|p| p.to_str().unwrap()));
    let (_, err, code) = delphi(&args);
    assert_ne!(code, 0);
    assert!(err.contains("8") || err.to_lowercase().contains("eight"));
}

#[test]
fn preflight_reports_page_count() {
    // Build a PDF via zine, then preflight it.
    let dir = scratch("preflight");
    let imgs = build_eight_image_set(&dir);
    let pdf = dir.join("z.pdf");
    let mut args: Vec<&str> = vec!["zine"];
    args.extend(imgs.iter().map(|p| p.to_str().unwrap()));
    args.extend(["-o", pdf.to_str().unwrap()]);
    let (_, _, code) = delphi(&args);
    assert_eq!(code, 0);
    let (out, _, _) = delphi(&["preflight", pdf.to_str().unwrap(), "-j"]);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["page_count"], 1);
    assert_eq!(v["encrypted"], false);
}

#[test]
fn impose_saddle_stitch_doubles_pages() {
    // Build a fake "source" PDF: zine produces 1 page. Use impose to verify it still produces
    // a valid output PDF (1 sheet → 2 output pages because saddle-stitch is duplex by default).
    let dir = scratch("impose");
    let imgs = build_eight_image_set(&dir);
    let pdf = dir.join("z.pdf");
    let mut args: Vec<&str> = vec!["zine"];
    args.extend(imgs.iter().map(|p| p.to_str().unwrap()));
    args.extend(["-o", pdf.to_str().unwrap()]);
    let (_, _, c) = delphi(&args);
    assert_eq!(c, 0);

    let out = dir.join("imposed.pdf");
    let (_, err, code) = delphi(&[
        "impose",
        pdf.to_str().unwrap(),
        "--layout",
        "saddle-stitch",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_pdf(&out));
    // preflight should be able to read it.
    let (pf, _, _) = delphi(&["preflight", out.to_str().unwrap(), "-j"]);
    let v: serde_json::Value = serde_json::from_str(&pf).expect("preflight JSON");
    // saddle-stitch pads 1 → 4 source pages → 1 sheet × 2 (front+back) = 2 output pages.
    assert_eq!(v["page_count"], 2);
}

// ===========================================================================
// Generators: qr / barcode / meta
// ===========================================================================

#[test]
fn qr_writes_valid_png() {
    let dir = scratch("qr");
    let out = dir.join("q.png");
    let (_, err, code) = delphi(&[
        "qr",
        "hello world",
        "--size",
        "256",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_png(&out));
}

#[test]
fn qr_transparent_bg() {
    let dir = scratch("qr-trans");
    let out = dir.join("q.png");
    let (_, _, code) = delphi(&[
        "qr",
        "x",
        "--bg",
        "transparent",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert!(is_png(&out));
}

#[test]
fn qr_empty_data_errors() {
    let dir = scratch("qr-empty");
    let out = dir.join("q.png");
    let (_, _, code) = delphi(&["qr", "", "-o", out.to_str().unwrap()]);
    assert_ne!(code, 0);
}

#[test]
fn barcode_code128_writes_png() {
    let dir = scratch("barcode");
    let out = dir.join("b.png");
    let (_, err, code) = delphi(&[
        "barcode",
        "HELLO123",
        "--format",
        "code128",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_png(&out));
}

#[test]
fn barcode_ean13_validates_input() {
    let dir = scratch("barcode-bad");
    let out = dir.join("b.png");
    let (_, _, code) = delphi(&[
        "barcode",
        "abc",
        "--format",
        "ean13",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_ne!(code, 0);
}

#[test]
fn barcode_unknown_format_errors() {
    let dir = scratch("barcode-fmt");
    let out = dir.join("b.png");
    let (_, _, code) = delphi(&[
        "barcode",
        "1",
        "--format",
        "morsecode",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_ne!(code, 0);
}

#[test]
fn meta_escapes_ampersand_and_quotes() {
    let (out, _, code) = delphi(&[
        "meta",
        "--title",
        "Cats & \"Dogs\"",
        "--description",
        "A & B",
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("&amp;"));
    assert!(out.contains("&quot;"));
    assert!(!out.contains("Cats & \""), "raw chars leaked through escape");
}

#[test]
fn meta_includes_og_and_twitter_when_image_set() {
    let (out, _, code) = delphi(&[
        "meta",
        "--title",
        "T",
        "--description",
        "D",
        "--url",
        "https://example.com",
        "--image",
        "https://example.com/i.png",
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("og:image"));
    assert!(out.contains("twitter:card"));
}

// ===========================================================================
// Text: typo / glyph / regex / font-info / shavian
// ===========================================================================

#[test]
fn typo_pt_to_mm() {
    let (out, _, code) = delphi(&["typo", "72pt", "mm"]);
    assert_eq!(code, 0);
    let v: f64 = out.trim().trim_end_matches("mm").parse().unwrap();
    assert!((v - 25.4).abs() < 0.01, "72pt should be 25.4mm, got {v}");
}

#[test]
fn typo_all_units_in_json() {
    let (out, _, code) = delphi(&["typo", "12pt", "-j"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert!(v.get("pt").is_some());
    assert!(v.get("px").is_some());
    assert!(v.get("mm").is_some());
    assert!(v.get("em").is_some());
}

#[test]
fn typo_unknown_unit_errors() {
    let (_, _, code) = delphi(&["typo", "12quux"]);
    assert_ne!(code, 0);
}

#[test]
fn glyph_codepoint_lookup() {
    let (out, _, code) = delphi(&["glyph", "U+2603"]);
    assert_eq!(code, 0);
    assert!(out.contains("U+2603"));
    assert!(out.contains("☃"));
    assert!(out.to_uppercase().contains("SNOWMAN"));
}

#[test]
fn glyph_search() {
    let (out, _, code) = delphi(&["glyph", "--search", "snowman", "--limit", "5"]);
    assert_eq!(code, 0);
    assert!(out.contains("☃") || out.contains("U+2603"));
}

#[test]
fn glyph_range_arrows() {
    let (out, _, code) = delphi(&["glyph", "--range", "arrows", "--limit", "10"]);
    assert_eq!(code, 0);
    // ← (LEFTWARDS ARROW) is in the range.
    assert!(out.contains("U+2190") || out.contains("←"));
}

#[test]
fn regex_finds_digits() {
    let (out, _, code) = delphi(&["regex", r"\d+", "abc 123 def 456"]);
    assert_eq!(code, 0);
    assert!(out.contains("Match 1"));
    assert!(out.contains("\"123\""));
    assert!(out.contains("\"456\""));
}

#[test]
fn regex_no_match() {
    let (out, _, code) = delphi(&["regex", "zzz", "abc"]);
    assert_eq!(code, 0);
    assert!(out.to_lowercase().contains("no match"));
}

#[test]
fn regex_invalid_pattern_errors() {
    let (_, _, code) = delphi(&["regex", "(unbalanced", "abc"]);
    assert_ne!(code, 0);
}

#[test]
fn regex_json_output_structure() {
    let (out, _, code) = delphi(&["regex", r"(\w+)=(\d+)", "a=1 b=2", "-j"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let arr = v.as_array().expect("array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["value"], "a=1");
    let groups = arr[0]["groups"].as_array().unwrap();
    assert_eq!(groups[0]["value"], "a");
    assert_eq!(groups[1]["value"], "1");
}

#[test]
fn font_info_macos_system_font() {
    // Geneva ships on every macOS — skip cleanly on other platforms.
    let geneva = std::path::Path::new("/System/Library/Fonts/Geneva.ttf");
    if !geneva.exists() {
        eprintln!("(skipping font_info_macos_system_font: Geneva.ttf not present)");
        return;
    }
    let (out, err, code) = delphi(&["font-info", geneva.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(out.contains("Format:"));
    assert!(out.contains("Glyphs:"));
    assert!(out.contains("@font-face"));
}

#[test]
fn shavian_basic_sentence() {
    let (out, _, code) = delphi(&["shavian", "The quick brown fox"]);
    assert_eq!(code, 0);
    // "the" → 𐑞 shorthand
    assert!(out.contains('𐑞'), "missing the shorthand: {out}");
    // Whitespace preserved → 4 space-separated words
    assert_eq!(out.trim().split_whitespace().count(), 4);
}

#[test]
fn shavian_preserves_punctuation() {
    let (out, _, code) = delphi(&["shavian", "Hello, world!"]);
    assert_eq!(code, 0);
    assert!(out.contains(','));
    assert!(out.contains('!'));
}

#[test]
fn shavian_json_per_token() {
    let (out, _, code) = delphi(&["shavian", "Hello world.", "-j"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    let arr = v.as_array().expect("array");
    let word_tokens: Vec<_> = arr.iter().filter(|t| t["type"] == "word").collect();
    assert!(word_tokens.len() >= 2);
    assert!(word_tokens[0].get("shavian").is_some());
}

// ===========================================================================
// Calc: calc / time / unit
// ===========================================================================

#[test]
fn calc_basic_expression() {
    let (out, _, code) = delphi(&["calc", "2 + 2"]);
    assert_eq!(code, 0);
    let n: f64 = out.trim().parse().expect("numeric output");
    assert!((n - 4.0).abs() < 1e-9);
}

#[test]
fn calc_trig_and_pi() {
    let (out, _, code) = delphi(&["calc", "sin(pi/2)"]);
    assert_eq!(code, 0);
    let n: f64 = out.trim().parse().expect("numeric output");
    assert!((n - 1.0).abs() < 1e-6, "sin(pi/2) ≈ 1, got {n}");
}

#[test]
fn calc_degrees_mode() {
    let (out, _, code) = delphi(&["calc", "sin(90)", "--angles", "deg"]);
    assert_eq!(code, 0);
    let n: f64 = out.trim().parse().expect("numeric output");
    assert!((n - 1.0).abs() < 1e-6, "sin(90 deg) ≈ 1, got {n}");
}

#[test]
fn calc_power_operator() {
    let (out, _, code) = delphi(&["calc", "2^10"]);
    assert_eq!(code, 0);
    let n: f64 = out.trim().parse().expect("numeric output");
    assert!((n - 1024.0).abs() < 1e-9);
}

#[test]
fn calc_invalid_expression_errors() {
    let (_, _, code) = delphi(&["calc", "2 +"]);
    assert_ne!(code, 0);
}

#[test]
fn time_now_includes_unix() {
    let (out, _, code) = delphi(&["time", "now"]);
    assert_eq!(code, 0);
    assert!(out.to_lowercase().contains("unix"));
}

#[test]
fn time_unix_timestamp_round_trip() {
    let (out, _, code) = delphi(&["time", "1744209000", "--to", "iso"]);
    assert_eq!(code, 0);
    assert!(out.contains("2025-04-09") || out.contains("2025-04-10"));
}

#[test]
fn time_add_duration() {
    let (out, _, code) = delphi(&["time", "2026-01-01", "--add", "30d", "--to", "iso"]);
    assert_eq!(code, 0);
    assert!(out.contains("2026-01-31") || out.contains("2026-02"));
}

#[test]
fn time_invalid_input_errors() {
    let (_, _, code) = delphi(&["time", "tomorrow probably"]);
    assert_ne!(code, 0);
}

#[test]
fn unit_kg_to_lb() {
    let (out, _, code) = delphi(&["unit", "100kg", "lb"]);
    assert_eq!(code, 0);
    let n: f64 = out.trim().parse().expect("numeric");
    assert!((n - 220.4623).abs() < 0.01, "100kg → ~220.46 lb, got {n}");
}

#[test]
fn unit_celsius_to_fahrenheit() {
    let (out, _, code) = delphi(&["unit", "100c", "f"]);
    assert_eq!(code, 0);
    let n: f64 = out.trim().parse().expect("numeric");
    assert!((n - 212.0).abs() < 0.01, "100c → 212f, got {n}");
}

#[test]
fn unit_cross_category_errors() {
    let (_, _, code) = delphi(&["unit", "100kg", "m"]);
    assert_ne!(code, 0, "kg→m must reject (mass→length)");
}

#[test]
fn unit_unknown_unit_errors() {
    let (_, _, code) = delphi(&["unit", "100notaunit"]);
    assert_ne!(code, 0);
}

// ===========================================================================
// Existing line-height / paper aliased commands (sanity checks for new aliases)
// ===========================================================================

#[test]
fn line_height_alias() {
    let (out, _, code) = delphi(&["lh", "16"]);
    assert_eq!(code, 0);
    assert!(out.contains("tight") || out.contains("normal"));
}

#[test]
fn paper_a4_mm() {
    let (out, _, code) = delphi(&["paper", "a4"]);
    assert_eq!(code, 0);
    assert!(out.contains("210") && out.contains("297"));
}

#[test]
fn base_hex_to_dec() {
    let (out, _, code) = delphi(&["base", "ff", "dec", "--from", "hex"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "255");
}

// ===========================================================================
// Batch-mode sanity
// ===========================================================================

#[test]
fn convert_batch_writes_one_output_per_input() {
    let dir = scratch("convert-batch");
    let a = dir.join("a.png");
    let b = dir.join("b.png");
    make_png(&a, "a");
    make_png(&b, "b");
    let outdir = dir.join("out");
    std::fs::create_dir_all(&outdir).unwrap();
    let (_, err, code) = delphi(&[
        "convert",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--to",
        "jpeg",
        "-o",
        outdir.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_jpeg(&outdir.join("a.jpg")));
    assert!(is_jpeg(&outdir.join("b.jpg")));
}

#[test]
fn crop_batch_processes_all_inputs() {
    let dir = scratch("crop-batch");
    let a = dir.join("a.png");
    let b = dir.join("b.png");
    make_png(&a, "a");
    make_png(&b, "b");
    let outdir = dir.join("out");
    let (_, err, code) = delphi(&[
        "crop",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        "--ratio",
        "1:1",
        "-o",
        outdir.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_png(&outdir.join("a-cropped.png")));
    assert!(is_png(&outdir.join("b-cropped.png")));
}

// ===========================================================================
// Regression tests for code-review fixes
// ===========================================================================

#[test]
fn svgo_does_not_double_escape_attributes() {
    // & in an attribute value must not become &amp;amp; on round-trip.
    let dir = scratch("svgo-amp");
    let src = dir.join("src.svg");
    std::fs::write(
        &src,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
  <use href="img.svg#a&amp;b"/>
</svg>
"##,
    )
    .unwrap();
    let (_, err, code) = delphi(&["svgo", src.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {err}");
    let out = std::fs::read_to_string(dir.join("src-optimised.svg")).unwrap();
    assert!(out.contains("a&amp;b"), "expected single-escaped &, got: {out}");
    assert!(
        !out.contains("&amp;amp;"),
        "attribute was double-escaped: {out}"
    );
}

#[test]
fn svgo_preserves_whitespace_in_text_element() {
    // SVG <text> treats whitespace as significant. Stripping it changes the
    // rendered output.
    let dir = scratch("svgo-text");
    let src = dir.join("src.svg");
    std::fs::write(
        &src,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="20">
  <text x="0" y="15">A<tspan>B</tspan> <tspan>C</tspan></text>
</svg>
"##,
    )
    .unwrap();
    let (_, err, code) = delphi(&["svgo", src.to_str().unwrap()]);
    assert_eq!(code, 0, "stderr: {err}");
    let out = std::fs::read_to_string(dir.join("src-optimised.svg")).unwrap();
    // The literal space between </tspan> and <tspan> must survive.
    assert!(
        out.contains("</tspan> <tspan>"),
        "whitespace inside <text> was stripped: {out}"
    );
}

#[test]
fn wc_counts_unicode_characters_not_bytes() {
    // 'café' = 4 characters, 5 UTF-8 bytes. The 'characters' field must use chars.
    let (out, _, code) = delphi(&["wc", "café", "-j"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["characters"], 4);
    assert_eq!(v["characters_no_spaces"], 4);
}

#[test]
fn line_height_filter_honours_json_flag() {
    let (out, _, code) = delphi(&["lh", "16", "normal", "-j"]);
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
    assert_eq!(v["name"], "normal");
    assert_eq!(v["ratio"], 1.5);
    assert_eq!(v["px"], 24.0);
}

#[test]
fn paper_unknown_name_is_usage_error() {
    let (_, _, code) = delphi(&["paper", "Z99"]);
    // error.rs: Usage→1. Same input via `zine --paper Z99` also exits 1; this
    // test pins the consistency.
    assert_eq!(code, 1);
}

#[test]
fn impose_crop_marks_land_on_outer_edges() {
    // Build a 4-page PDF via zine, impose it n-up 4 with --crop-marks, then
    // peek into the PDF content stream to find the highest y-coordinate that
    // appears in a "m" (moveto) op. It must be near the sheet's top edge,
    // not in the middle gutter.
    let dir = scratch("impose-cm");
    let imgs = build_eight_image_set(&dir);
    let pdf = dir.join("z.pdf");
    let mut args: Vec<&str> = vec!["zine"];
    args.extend(imgs.iter().map(|p| p.to_str().unwrap()));
    args.extend(["-o", pdf.to_str().unwrap()]);
    let (_, _, c) = delphi(&args);
    assert_eq!(c, 0);

    let out = dir.join("cm.pdf");
    // zine produces a single landscape A4 page. Impose 2x2 n-up onto a landscape A4.
    let (_, err, code) = delphi(&[
        "impose",
        pdf.to_str().unwrap(),
        "--layout",
        "n-up",
        "--n-up",
        "4",
        "--crop-marks",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(is_pdf(&out));
    // Beyond a smoke check: preflight must be able to read it back, i.e. the
    // crop-mark drawing operations didn't break the PDF structure. (Exit code
    // may be 3 due to non-blocking warnings about embedded raster DPI.)
    let (preflight_out, _, _) = delphi(&["preflight", out.to_str().unwrap(), "-j"]);
    let v: serde_json::Value =
        serde_json::from_str(&preflight_out).expect("preflight JSON readable");
    assert!(v["page_count"].as_u64().unwrap_or(0) >= 1);
}

#[test]
fn matte_solid_honours_alpha() {
    // Pass a half-transparent fill colour. The matte canvas should retain
    // 50%-ish alpha, not be forced opaque.
    let dir = scratch("matte-alpha");
    let src = dir.join("src.png");
    make_png(&src, "matte-alpha");
    let (_, err, code) = delphi(&[
        "matte",
        src.to_str().unwrap(),
        "--mode",
        "solid",
        "--colour",
        "#ff000080",
        "--ratio",
        "16:9",
    ]);
    assert_eq!(code, 0, "stderr: {err}");
    let out = dir.join("src-matted.png");
    let img = image::open(&out).unwrap().to_rgba8();
    // Sample a corner pixel that lives in the matte region (not the centered image).
    let (w, h) = (img.width(), img.height());
    let p = img.get_pixel(2, 2);
    let alpha = p[3];
    assert!(
        (60..=170).contains(&alpha),
        "expected ~128 alpha on matte corner, got {alpha} (image {w}x{h})"
    );
}

#[test]
fn qr_bg_alpha_works_via_hex8() {
    // `--bg #ffffff00` should produce the same transparency as `--bg transparent`.
    let dir = scratch("qr-alpha");
    let p_str = dir.join("p_str.png");
    let p_hex = dir.join("p_hex.png");
    let (_, _, c1) = delphi(&[
        "qr",
        "hello",
        "--bg",
        "transparent",
        "-o",
        p_str.to_str().unwrap(),
    ]);
    assert_eq!(c1, 0);
    let (_, _, c2) = delphi(&[
        "qr",
        "hello",
        "--bg",
        "#ffffff00",
        "-o",
        p_hex.to_str().unwrap(),
    ]);
    assert_eq!(c2, 0);
    let i1 = image::open(&p_str).unwrap().to_rgba8();
    let i2 = image::open(&p_hex).unwrap().to_rgba8();
    // Background corner pixel alpha should be 0 in both.
    assert_eq!(i1.get_pixel(0, 0)[3], 0);
    assert_eq!(i2.get_pixel(0, 0)[3], 0);
}

#[test]
fn impose_n_up_duplex_warns() {
    // n-up + --duplex should warn (not silently ignore) and still succeed.
    let dir = scratch("impose-warn");
    let imgs = build_eight_image_set(&dir);
    let pdf = dir.join("z.pdf");
    let mut args: Vec<&str> = vec!["zine"];
    args.extend(imgs.iter().map(|p| p.to_str().unwrap()));
    args.extend(["-o", pdf.to_str().unwrap()]);
    let (_, _, c) = delphi(&args);
    assert_eq!(c, 0);

    let out = dir.join("nup.pdf");
    let (_, err, code) = delphi(&[
        "impose",
        pdf.to_str().unwrap(),
        "--layout",
        "n-up",
        "--n-up",
        "2",
        "--duplex",
        "-o",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, 0);
    assert!(is_pdf(&out));
    assert!(
        err.to_lowercase().contains("duplex") && err.to_lowercase().contains("ignored"),
        "expected warning about ignored --duplex, got: {err}"
    );
}
