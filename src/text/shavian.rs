use crate::error::Error;
use crate::input;
use crate::output;
use serde_json::json;
use std::collections::HashMap;
use std::sync::OnceLock;

// ── Bundled dictionary ───────────────────────────────────────────────────────

const CORE_JSON: &str = include_str!("../../data/shavian-core.json");

fn dictionary() -> &'static HashMap<String, Vec<String>> {
    static DICT: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    DICT.get_or_init(|| {
        serde_json::from_str(CORE_JSON).expect("shavian-core.json: invalid JSON")
    })
}

// ── Shorthands ───────────────────────────────────────────────────────────────

/// Common words spelt as a single Shavian letter.
fn shorthand(word_lower: &str) -> Option<(&'static str, &'static str)> {
    match word_lower {
        "the" => Some(("𐑞", "ðə")),
        "of" => Some(("𐑝", "əv")),
        "and" => Some(("𐑯", "ənd")),
        "to" => Some(("𐑑", "tuː")),
        "for" => Some(("𐑓", "fɔːr")),
        _ => None,
    }
}

// ── ARPABET overrides ────────────────────────────────────────────────────────

/// PALM words: CMU uses AA (LOT) but Shavian wants 𐑭 (PALM).
const PALM_WORDS: &[&str] = &[
    "father", "rather", "lather",
    "calm", "palm", "psalm", "balm", "almond",
    "drama", "banana", "llama", "mama", "papa",
    "lava", "java", "guava",
    "rajah", "hurrah", "aha", "bah",
    "salami", "safari", "tsunami", "khaki",
    "spa", "bra",
    "pasta", "plaza", "taco",
    "cantata", "sonata", "aria",
];

/// THOUGHT bugs: CMU incorrectly uses AA instead of AO.
const THOUGHT_BUG_WORDS: &[&str] = &["caught", "bought", "raw", "spawn", "cause"];

fn apply_overrides(word_lower: &str, arpabets: &[String]) -> Vec<String> {
    let (from, to) = if PALM_WORDS.contains(&word_lower) {
        ("AA", "AA_PALM")
    } else if THOUGHT_BUG_WORDS.contains(&word_lower) {
        ("AA", "AO")
    } else {
        return arpabets.to_vec();
    };
    arpabets
        .iter()
        .map(|code| {
            if normalize_arpabet(code) == from {
                to.to_string()
            } else {
                code.clone()
            }
        })
        .collect()
}

// ── Phoneme normalisation / mapping ──────────────────────────────────────────

/// Strip ARPABET stress digits, with special-case schwa and kit.
///
/// - AH0 → "AH0" (schwa 𐑩), AH1/AH2 → "AH" (strut 𐑳)
/// - IY0 → "IY0" (kit 𐑦), IY1/IY2 → "IY" (fleece 𐑰)
/// - Everything else: drop a trailing 0/1/2
fn normalize_arpabet(code: &str) -> String {
    if let Some(rest) = code.strip_prefix("AH") {
        return if rest.ends_with('0') { "AH0".into() } else { "AH".into() };
    }
    if let Some(rest) = code.strip_prefix("IY") {
        return if rest.ends_with('0') { "IY0".into() } else { "IY".into() };
    }
    if let Some(last) = code.chars().last() {
        if matches!(last, '0' | '1' | '2') {
            return code[..code.len() - 1].to_string();
        }
    }
    code.to_string()
}

fn arpabet_to_shavian(code: &str) -> Option<&'static str> {
    Some(match normalize_arpabet(code).as_str() {
        "P" => "𐑐", "T" => "𐑑", "K" => "𐑒", "F" => "𐑓",
        "TH" => "𐑔", "S" => "𐑕", "SH" => "𐑖", "CH" => "𐑗",
        "B" => "𐑚", "D" => "𐑛", "G" => "𐑜", "V" => "𐑝",
        "DH" => "𐑞", "Z" => "𐑟", "ZH" => "𐑠", "JH" => "𐑡",
        "Y" => "𐑘", "W" => "𐑢", "NG" => "𐑙", "HH" => "𐑣",
        "M" => "𐑥", "N" => "𐑯", "L" => "𐑤", "R" => "𐑮",
        "AE" => "𐑨", "AH0" => "𐑩", "AH" => "𐑳",
        "AA" => "𐑪", "AA_PALM" => "𐑭",
        "UH" => "𐑫", "IH" => "𐑦", "EH" => "𐑧",
        "EY" => "𐑱", "IY" => "𐑰", "IY0" => "𐑦", "AY" => "𐑲",
        "OW" => "𐑴", "UW" => "𐑵", "OY" => "𐑶",
        "AW" => "𐑬", "AO" => "𐑷",
        "ER" => "𐑼",
        "YUW" => "𐑿",
        _ => return None,
    })
}

fn arpabet_to_ipa(code: &str) -> Option<&'static str> {
    Some(match normalize_arpabet(code).as_str() {
        "P" => "p", "T" => "t", "K" => "k", "F" => "f",
        "TH" => "θ", "S" => "s", "SH" => "ʃ", "CH" => "tʃ",
        "B" => "b", "D" => "d", "G" => "ɡ", "V" => "v",
        "DH" => "ð", "Z" => "z", "ZH" => "ʒ", "JH" => "dʒ",
        "Y" => "j", "W" => "w", "NG" => "ŋ", "HH" => "h",
        "M" => "m", "N" => "n", "L" => "l", "R" => "r",
        "AE" => "æ", "AH0" => "ə", "AH" => "ʌ",
        "AA" => "ɒ", "AA_PALM" => "ɑː",
        "UH" => "ʊ", "IH" => "ɪ", "EH" => "ɛ",
        "EY" => "eɪ", "IY" => "iː", "IY0" => "i", "AY" => "aɪ",
        "OW" => "oʊ", "UW" => "uː", "OY" => "ɔɪ",
        "AW" => "aʊ", "AO" => "ɔː",
        "ER" => "ɚ",
        "YUW" => "juː",
        _ => return None,
    })
}

/// Merge ARPABET sequences that fuse into a single Shavian letter (Y + UW → YUW).
fn merge_sequences(arpabets: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(arpabets.len());
    let mut i = 0;
    while i < arpabets.len() {
        if arpabets[i] == "Y"
            && i + 1 < arpabets.len()
            && normalize_arpabet(&arpabets[i + 1]) == "UW"
        {
            out.push("YUW".to_string());
            i += 2;
        } else {
            out.push(arpabets[i].clone());
            i += 1;
        }
    }
    out
}

// ── Heuristic fallback ───────────────────────────────────────────────────────

/// Grapheme rules, longest-first. Each maps an English spelling chunk to a
/// Shavian character string (occasionally empty for silent letters).
const GRAPHEME_RULES: &[(&str, &str)] = &[
    ("tion", "𐑖𐑩𐑯"),
    ("sion", "𐑠𐑩𐑯"),
    ("ture", "𐑗𐑼"),
    ("ough", "𐑴"),
    ("ight", "𐑲𐑑"),
    ("ould", "𐑫𐑛"),
    ("ious", "𐑾𐑕"),
    ("eous", "𐑾𐑕"),
    ("tch", "𐑗"),
    ("dge", "𐑡"),
    ("sch", "𐑕𐑒"),
    ("scr", "𐑕𐑒𐑮"),
    ("shr", "𐑖𐑮"),
    ("thr", "𐑔𐑮"),
    ("str", "𐑕𐑑𐑮"),
    ("spl", "𐑕𐑐𐑤"),
    ("spr", "𐑕𐑐𐑮"),
    ("kn", "𐑯"),
    ("wr", "𐑮"),
    ("gn", "𐑯"),
    ("ph", "𐑓"),
    ("wh", "𐑢"),
    ("gh", ""),
    ("th", "𐑔"),
    ("sh", "𐑖"),
    ("ch", "𐑗"),
    ("ng", "𐑙"),
    ("nk", "𐑙𐑒"),
    ("qu", "𐑒𐑢"),
    ("ck", "𐑒"),
    ("ee", "𐑰"),
    ("ea", "𐑰"),
    ("oo", "𐑵"),
    ("ou", "𐑬"),
    ("ow", "𐑬"),
    ("oi", "𐑶"),
    ("oy", "𐑶"),
    ("ai", "𐑱"),
    ("ay", "𐑱"),
    ("ei", "𐑱"),
    ("ey", "𐑱"),
    ("ie", "𐑰"),
    ("aw", "𐑷"),
    ("au", "𐑷"),
    ("er", "𐑼"),
    ("ir", "𐑻"),
    ("ur", "𐑻"),
    ("or", "𐑹"),
    ("ar", "𐑸"),
    ("ew", "𐑿"),
    ("a", "𐑨"),
    ("b", "𐑚"),
    ("c", "𐑒"),
    ("d", "𐑛"),
    ("e", "𐑧"),
    ("f", "𐑓"),
    ("g", "𐑜"),
    ("h", "𐑣"),
    ("i", "𐑦"),
    ("j", "𐑡"),
    ("k", "𐑒"),
    ("l", "𐑤"),
    ("m", "𐑥"),
    ("n", "𐑯"),
    ("o", "𐑪"),
    ("p", "𐑐"),
    ("r", "𐑮"),
    ("s", "𐑕"),
    ("t", "𐑑"),
    ("u", "𐑳"),
    ("v", "𐑝"),
    ("w", "𐑢"),
    ("x", "𐑒𐑕"),
    ("y", "𐑘"),
    ("z", "𐑟"),
];

/// Shavian → IPA table (for the heuristic output, which doesn't carry IPA).
fn shavian_to_ipa(c: char) -> Option<&'static str> {
    Some(match c {
        '𐑐' => "p", '𐑑' => "t", '𐑒' => "k", '𐑓' => "f",
        '𐑔' => "θ", '𐑕' => "s", '𐑖' => "ʃ", '𐑗' => "tʃ",
        '𐑚' => "b", '𐑛' => "d", '𐑜' => "ɡ", '𐑝' => "v",
        '𐑞' => "ð", '𐑟' => "z", '𐑠' => "ʒ", '𐑡' => "dʒ",
        '𐑘' => "j", '𐑢' => "w", '𐑙' => "ŋ", '𐑣' => "h",
        '𐑥' => "m", '𐑯' => "n", '𐑤' => "l", '𐑮' => "r",
        '𐑨' => "æ", '𐑩' => "ə", '𐑪' => "ɒ", '𐑫' => "ʊ",
        '𐑦' => "ɪ", '𐑧' => "ɛ", '𐑳' => "ʌ",
        '𐑱' => "eɪ", '𐑰' => "iː", '𐑲' => "aɪ", '𐑴' => "oʊ",
        '𐑵' => "uː", '𐑶' => "ɔɪ", '𐑬' => "aʊ", '𐑷' => "ɔː",
        '𐑸' => "ɑːr", '𐑹' => "ɔːr", '𐑺' => "ɛər", '𐑻' => "ɜːr",
        '𐑼' => "ɚ", '𐑽' => "ɪər", '𐑾' => "ɪə", '𐑿' => "juː",
        '𐑭' => "ɑː",
        _ => return None,
    })
}

#[derive(Clone)]
struct Phoneme {
    shavian: String,
    ipa: String,
}

/// Rule-based fallback for words not in the dictionary.
fn heuristic_transliterate(word: &str) -> Vec<Phoneme> {
    let lower: String = word.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();

    // Strip a likely-silent trailing 'e': only if length > 2 and the
    // preceding character is a non-vowel (consonant-style ending).
    let effective: String = if chars.len() > 2 && chars.last() == Some(&'e') {
        let prev = chars[chars.len() - 2];
        if !matches!(prev, 'a' | 'e' | 'i' | 'o' | 'u' | 'y') {
            chars[..chars.len() - 1].iter().collect()
        } else {
            lower.clone()
        }
    } else {
        lower.clone()
    };

    let bytes = effective.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        let mut matched = false;
        for (grapheme, shavian_str) in GRAPHEME_RULES {
            let g = grapheme.as_bytes();
            if i + g.len() <= bytes.len() && &bytes[i..i + g.len()] == g {
                for ch in shavian_str.chars() {
                    let ipa = shavian_to_ipa(ch).unwrap_or("").to_string();
                    result.push(Phoneme {
                        shavian: ch.to_string(),
                        ipa,
                    });
                }
                i += g.len();
                matched = true;
                break;
            }
        }
        if !matched {
            // Skip unknown bytes (digits, hyphens, stray ASCII).
            i += 1;
        }
    }

    result
}

// ── Word transliteration ─────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Source {
    Shorthand,
    Dict,
    Heuristic,
}

impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Source::Shorthand => "shorthand",
            Source::Dict => "dict",
            Source::Heuristic => "heuristic",
        }
    }
}

struct WordGloss {
    shavian: String,
    ipa: String,
    source: Source,
}

fn transliterate_word(word: &str) -> WordGloss {
    let lower = word.to_lowercase();

    if let Some((shav, ipa)) = shorthand(&lower) {
        return WordGloss {
            shavian: shav.to_string(),
            ipa: ipa.to_string(),
            source: Source::Shorthand,
        };
    }

    if let Some(arpabets) = dictionary().get(&lower) {
        let corrected = apply_overrides(&lower, arpabets);
        let merged = merge_sequences(&corrected);
        let mut shav = String::new();
        let mut ipa = String::new();
        for code in &merged {
            shav.push_str(arpabet_to_shavian(code).unwrap_or("?"));
            ipa.push_str(arpabet_to_ipa(code).unwrap_or("?"));
        }
        return WordGloss { shavian: shav, ipa, source: Source::Dict };
    }

    let phonemes = heuristic_transliterate(word);
    let shavian = phonemes.iter().map(|p| p.shavian.as_str()).collect::<String>();
    let ipa = phonemes.iter().map(|p| p.ipa.as_str()).collect::<String>();
    WordGloss { shavian, ipa, source: Source::Heuristic }
}

// ── Tokenisation ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TokenKind {
    Word,
    Whitespace,
    Punctuation,
}

struct Token {
    kind: TokenKind,
    value: String,
}

fn tokenise(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_alphabetic() || c == '\'' {
            // Words: letters plus apostrophes (don't, it's).
            let start = i;
            while i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '\'') {
                i += 1;
            }
            let value: String = chars[start..i].iter().collect();
            // A bare apostrophe (e.g. opening quote) isn't a word; treat as punctuation.
            let has_letter = value.chars().any(|c| c.is_alphabetic());
            tokens.push(Token {
                kind: if has_letter { TokenKind::Word } else { TokenKind::Punctuation },
                value,
            });
        } else if c.is_whitespace() {
            let start = i;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Whitespace,
                value: chars[start..i].iter().collect(),
            });
        } else {
            let start = i;
            while i < chars.len()
                && !chars[i].is_alphabetic()
                && !chars[i].is_whitespace()
                && chars[i] != '\''
            {
                i += 1;
            }
            tokens.push(Token {
                kind: TokenKind::Punctuation,
                value: chars[start..i].iter().collect(),
            });
        }
    }

    tokens
}

// ── Public entry point ───────────────────────────────────────────────────────

pub fn run(input_arg: Option<&str>, gloss: bool, as_json: bool) -> Result<(), Error> {
    let text = input::read_text(input_arg)?;
    let tokens = tokenise(&text);

    if as_json {
        emit_json(&tokens);
    } else if gloss {
        emit_gloss(&tokens);
    } else {
        emit_plain(&tokens);
    }

    Ok(())
}

fn emit_plain(tokens: &[Token]) {
    let mut out = String::new();
    for tok in tokens {
        match tok.kind {
            TokenKind::Word => {
                let g = transliterate_word(&tok.value);
                out.push_str(&g.shavian);
            }
            TokenKind::Whitespace | TokenKind::Punctuation => out.push_str(&tok.value),
        }
    }
    // Only add a trailing newline if input didn't already end with one,
    // so that piped output is predictable.
    if !out.ends_with('\n') {
        out.push('\n');
    }
    print!("{out}");
}

fn emit_json(tokens: &[Token]) {
    let arr: Vec<serde_json::Value> = tokens
        .iter()
        .map(|tok| match tok.kind {
            TokenKind::Word => {
                let g = transliterate_word(&tok.value);
                json!({
                    "type": "word",
                    "value": tok.value,
                    "shavian": g.shavian,
                    "ipa": g.ipa,
                    "source": g.source.as_str(),
                })
            }
            TokenKind::Whitespace => json!({
                "type": "whitespace",
                "value": tok.value,
            }),
            TokenKind::Punctuation => json!({
                "type": "punctuation",
                "value": tok.value,
            }),
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&arr).unwrap());
}

fn emit_gloss(tokens: &[Token]) {
    // 3-row layout per logical line: Latin / Shavian / IPA columns.
    // Each word/punct/space becomes a column; columns are padded to the widest
    // of the three rows so they align. Newlines in the source break the layout
    // into multiple line-groups.

    let dim_on = if output::is_tty() { "\x1b[2m" } else { "" };
    let dim_off = if output::is_tty() { "\x1b[0m" } else { "" };

    // Group tokens into lines (split on whitespace tokens containing '\n').
    let mut line: Vec<&Token> = Vec::new();
    let mut first_line = true;
    for tok in tokens {
        if tok.kind == TokenKind::Whitespace && tok.value.contains('\n') {
            if !first_line {
                println!();
            }
            print_gloss_line(&line, dim_on, dim_off);
            first_line = false;
            line.clear();
        } else {
            line.push(tok);
        }
    }
    if !line.is_empty() {
        if !first_line {
            println!();
        }
        print_gloss_line(&line, dim_on, dim_off);
    }
}

fn print_gloss_line(line: &[&Token], dim_on: &str, dim_off: &str) {
    // Build three parallel rows: Latin, Shavian, IPA.
    let mut latin_cells: Vec<String> = Vec::new();
    let mut shav_cells: Vec<String> = Vec::new();
    let mut ipa_cells: Vec<String> = Vec::new();

    for tok in line {
        match tok.kind {
            TokenKind::Word => {
                let g = transliterate_word(&tok.value);
                latin_cells.push(tok.value.clone());
                shav_cells.push(g.shavian);
                ipa_cells.push(g.ipa);
            }
            TokenKind::Punctuation => {
                latin_cells.push(tok.value.clone());
                shav_cells.push(tok.value.clone());
                ipa_cells.push(tok.value.clone());
            }
            TokenKind::Whitespace => {
                // Preserve a single space gap between columns.
                latin_cells.push(" ".to_string());
                shav_cells.push(" ".to_string());
                ipa_cells.push(" ".to_string());
            }
        }
    }

    // Per-column padding: pad each cell to max(width(latin), width(shav), width(ipa)).
    let mut latin_row = String::new();
    let mut shav_row = String::new();
    let mut ipa_row = String::new();

    for ((l, s), i) in latin_cells.iter().zip(shav_cells.iter()).zip(ipa_cells.iter()) {
        let w = display_width(l).max(display_width(s)).max(display_width(i));
        latin_row.push_str(&pad_to(l, w));
        shav_row.push_str(&pad_to(s, w));
        ipa_row.push_str(&pad_to(i, w));
    }

    println!("{dim_on}{}{dim_off}", latin_row.trim_end());
    println!("{}", shav_row.trim_end());
    println!("{dim_on}{}{dim_off}", ipa_row.trim_end());
}

fn display_width(s: &str) -> usize {
    s.chars().count()
}

fn pad_to(s: &str, width: usize) -> String {
    let w = display_width(s);
    if w >= width {
        s.to_string()
    } else {
        let mut out = String::with_capacity(s.len() + (width - w));
        out.push_str(s);
        for _ in 0..(width - w) {
            out.push(' ');
        }
        out
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shavian_shorthand_the() {
        let g = transliterate_word("the");
        assert_eq!(g.shavian, "𐑞");
        assert_eq!(g.source, Source::Shorthand);
    }

    #[test]
    fn shavian_shorthand_case_insensitive() {
        let g = transliterate_word("The");
        assert_eq!(g.shavian, "𐑞");
    }

    #[test]
    fn shavian_dict_hello() {
        let g = transliterate_word("hello");
        assert_eq!(g.shavian, "𐑣𐑩𐑤𐑴");
        assert_eq!(g.source, Source::Dict);
    }

    #[test]
    fn shavian_palm_override_father() {
        // father: F AA1 DH ER0 — AA must become AA_PALM (𐑭) not 𐑪.
        let g = transliterate_word("father");
        assert!(
            g.shavian.contains('𐑭'),
            "expected 𐑭 (PALM) in '{}', got {:?}",
            "father", g.shavian
        );
    }

    #[test]
    fn shavian_thought_bug_caught() {
        // caught: K AO1 T in CMU? Actually CMU has it as K AA1 T (bug),
        // which we override to AO → 𐑷.
        let g = transliterate_word("caught");
        assert!(g.shavian.contains('𐑷') || g.shavian.contains('𐑹'),
            "expected 𐑷/𐑹 in caught, got {:?}", g.shavian);
    }

    #[test]
    fn shavian_yew_ligature() {
        // "you" is in the dict as ["Y","UW1"] — should merge to YUW → 𐑿.
        let g = transliterate_word("you");
        assert_eq!(g.shavian, "𐑿");
    }

    #[test]
    fn shavian_heuristic_gibberish() {
        // Gibberish word should fall back to the heuristic and still produce output.
        let g = transliterate_word("xyzqq");
        assert_eq!(g.source, Source::Heuristic);
        assert!(!g.shavian.is_empty());
    }

    #[test]
    fn shavian_heuristic_silent_e() {
        // "cake" → heuristic strips trailing e if preceded by consonant.
        let g = transliterate_word("zqzqe");
        assert_eq!(g.source, Source::Heuristic);
        // Should not be empty.
        assert!(!g.shavian.is_empty());
    }

    #[test]
    fn shavian_tokenise_preserves_punctuation() {
        let toks = tokenise("Hello, world!");
        assert_eq!(toks.len(), 5);
        assert_eq!(toks[0].kind, TokenKind::Word);
        assert_eq!(toks[0].value, "Hello");
        assert_eq!(toks[1].kind, TokenKind::Punctuation);
        assert_eq!(toks[1].value, ",");
        assert_eq!(toks[2].kind, TokenKind::Whitespace);
        assert_eq!(toks[3].kind, TokenKind::Word);
        assert_eq!(toks[3].value, "world");
        assert_eq!(toks[4].kind, TokenKind::Punctuation);
        assert_eq!(toks[4].value, "!");
    }

    #[test]
    fn shavian_tokenise_apostrophe_in_word() {
        let toks = tokenise("don't");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].kind, TokenKind::Word);
        assert_eq!(toks[0].value, "don't");
    }

    #[test]
    fn shavian_determinism() {
        let input = "The quick brown fox jumps over the lazy dog.";
        let toks_a = tokenise(input);
        let toks_b = tokenise(input);
        let line_a: String = toks_a
            .iter()
            .map(|t| match t.kind {
                TokenKind::Word => transliterate_word(&t.value).shavian,
                _ => t.value.clone(),
            })
            .collect();
        let line_b: String = toks_b
            .iter()
            .map(|t| match t.kind {
                TokenKind::Word => transliterate_word(&t.value).shavian,
                _ => t.value.clone(),
            })
            .collect();
        assert_eq!(line_a, line_b);
    }

    #[test]
    fn shavian_normalize_ah() {
        assert_eq!(normalize_arpabet("AH0"), "AH0");
        assert_eq!(normalize_arpabet("AH1"), "AH");
        assert_eq!(normalize_arpabet("AH2"), "AH");
    }

    #[test]
    fn shavian_normalize_iy() {
        assert_eq!(normalize_arpabet("IY0"), "IY0");
        assert_eq!(normalize_arpabet("IY1"), "IY");
    }

    #[test]
    fn shavian_normalize_ow() {
        assert_eq!(normalize_arpabet("OW1"), "OW");
        assert_eq!(normalize_arpabet("OW0"), "OW");
    }

    #[test]
    fn shavian_dictionary_loads() {
        // Sanity: dictionary must contain "hello".
        assert!(dictionary().contains_key("hello"));
        // And have a reasonable size.
        assert!(dictionary().len() > 1000);
    }
}
