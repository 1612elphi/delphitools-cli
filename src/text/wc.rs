use crate::error::Error;
use crate::input;
use serde_json::json;

pub fn run(arg: Option<&str>, as_json: bool) -> Result<(), Error> {
    let text = input::read_text(arg)?;

    // Count Unicode scalar values, not UTF-8 bytes — `chars_no_spaces` uses
    // the same unit, and any other choice would make the two fields disagree
    // (e.g. "café" is 4 chars / 5 bytes).
    let chars = text.chars().count();
    let chars_no_spaces = text.chars().filter(|c| !c.is_whitespace()).count();
    let words = text.split_whitespace().count();
    let lines = if text.is_empty() {
        0
    } else {
        text.lines().count()
    };
    let paragraphs = if text.is_empty() {
        0
    } else {
        text.split("\n\n").filter(|p| !p.trim().is_empty()).count()
    };
    let sentences = text
        .chars()
        .filter(|c| matches!(c, '.' | '!' | '?'))
        .count()
        .max(if words > 0 { 1 } else { 0 });

    let read_min = (words as f64 / 200.0).ceil() as usize;
    let speak_min = (words as f64 / 150.0).ceil() as usize;

    if as_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "words": words,
                "characters": chars,
                "characters_no_spaces": chars_no_spaces,
                "sentences": sentences,
                "paragraphs": paragraphs,
                "lines": lines,
                "reading_minutes": read_min,
                "speaking_minutes": speak_min,
            }))
            .unwrap()
        );
    } else {
        println!(
            "Words: {}  Characters: {} ({} without spaces)  Sentences: {}",
            words, chars, chars_no_spaces, sentences
        );
        println!("Paragraphs: {}  Lines: {}", paragraphs, lines);
        println!("Reading: ~{read_min} min  Speaking: ~{speak_min} min");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    fn count(text: &str) -> (usize, usize, usize) {
        let words = text.split_whitespace().count();
        let chars = text.len();
        let sentences = text
            .chars()
            .filter(|c| matches!(c, '.' | '!' | '?'))
            .count()
            .max(if words > 0 { 1 } else { 0 });
        (words, chars, sentences)
    }

    #[test]
    fn basic_sentence() {
        let (w, c, s) = count("The quick brown fox.");
        assert_eq!(w, 4);
        assert_eq!(c, 20);
        assert_eq!(s, 1);
    }

    #[test]
    fn multiple_sentences() {
        let (w, _, s) = count("Hello world. How are you? Fine!");
        assert_eq!(w, 6);
        assert_eq!(s, 3);
    }

    #[test]
    fn empty_string() {
        let (w, c, s) = count("");
        assert_eq!(w, 0);
        assert_eq!(c, 0);
        assert_eq!(s, 0);
    }

    #[test]
    fn no_punctuation_still_one_sentence() {
        let (w, _, s) = count("hello world");
        assert_eq!(w, 2);
        assert_eq!(s, 1); // at least 1 if there are words
    }

    #[test]
    fn reading_time() {
        let words = 400;
        let read_min = (words as f64 / 200.0).ceil() as usize;
        let speak_min = (words as f64 / 150.0).ceil() as usize;
        assert_eq!(read_min, 2);
        assert_eq!(speak_min, 3);
    }
}
