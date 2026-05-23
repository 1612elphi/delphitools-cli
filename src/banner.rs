//! Coloured ASCII-art banner + bare-invocation "tasting menu". A fresh cohesive
//! random palette is sampled per run so every load is a different colourway.

use crate::colour::palette::Strategy;
use crate::colour::Colour;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::io::IsTerminal;

const ART: &str = "     __    __     __   _ __            __  \n \
                   ___/ /__ / /__  / /  (_) /____  ___  / /__\n\
                   / _  / -_) / _ \\/ _ \\/ / __/ _ \\/ _ \\/ (_-<\n\
                   \\_,_/\\__/_/ .__/_//_/_/\\__/\\___/\\___/_/___/\n         \
                   /_/                               ";

const TAGLINE: &str = "delphitools — indie toolkit";

/// Render just the wordmark (without the tagline). Used by clap's `--help`
/// output via `before_help`. Returns ANSI-coloured text when running in a TTY.
pub fn render() -> String {
    let want_colour = should_colour();
    let mut rng = thread_rng();
    let palette = Strategy::RandomCohesive.generate(5, &mut rng);
    paint_wordmark(&palette, want_colour)
}

/// Print the bare-invocation "tasting menu": wordmark, tagline, eight randomly
/// chosen tools with descriptions, plus a hint about `delphi ?`. Each run
/// resamples both the palette *and* which tools get featured.
pub fn print_sampler(commands: &[(String, String, Vec<String>)]) {
    let want_colour = should_colour();
    let mut rng = thread_rng();

    // Generate 8 cohesive colours: first 5 paint the wordmark lines, then the
    // tool list cycles through all 8.
    let palette = Strategy::RandomCohesive.generate(8, &mut rng);
    let wordmark = paint_wordmark(&palette[..5], want_colour);
    print!("{wordmark}");
    println!();
    println!("  {TAGLINE}");
    println!();

    // Sample 8 commands without replacement.
    let mut pool: Vec<&(String, String, Vec<String>)> = commands.iter().collect();
    pool.shuffle(&mut rng);
    let sample: Vec<_> = pool.into_iter().take(8).collect();

    let name_width = sample.iter().map(|(n, _, _)| n.len()).max().unwrap_or(0);

    for (i, (name, about, aliases)) in sample.iter().enumerate() {
        let colour = &palette[i % palette.len()];
        let alias_hint = if aliases.is_empty() {
            String::new()
        } else {
            format!("  ({})", aliases.join(", "))
        };
        if want_colour {
            let (r, g, b) = colour.to_u8();
            let dim = "\x1b[2m";
            let reset = "\x1b[0m";
            println!(
                "  \x1b[38;2;{r};{g};{b}m{name:<width$}\x1b[0m  {about}{dim}{alias_hint}{reset}",
                width = name_width
            );
        } else {
            println!("  {name:<width$}  {about}{alias_hint}", width = name_width);
        }
    }

    println!();
    if want_colour {
        let (r, g, b) = palette[0].to_u8();
        let dim = "\x1b[2m";
        let reset = "\x1b[0m";
        println!(
            "  {dim}Run{reset} \x1b[38;2;{r};{g};{b}mdelphi ?\x1b[0m {dim}for the full list, or{reset} \x1b[38;2;{r};{g};{b}mdelphi <command> --help\x1b[0m{dim} for usage.{reset}"
        );
        println!(
            "  {dim}If you're an AI agent, run{reset} \x1b[38;2;{r};{g};{b}mdelphi agent\x1b[0m {dim}for a machine-readable reference.{reset}"
        );
    } else {
        println!("  Run `delphi ?` for the full list, or `delphi <command> --help` for usage.");
        println!("  If you're an AI agent, run `delphi agent` for a machine-readable reference.");
    }
}

fn paint_wordmark(palette: &[Colour], want_colour: bool) -> String {
    let lines: Vec<&str> = ART.lines().collect();
    let mut out = String::new();
    if want_colour && !palette.is_empty() {
        for (i, line) in lines.iter().enumerate() {
            let c = &palette[i % palette.len()];
            let (r, g, b) = c.to_u8();
            out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{line}\x1b[0m\n"));
        }
    } else {
        for line in &lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn should_colour() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal() || std::io::stderr().is_terminal()
}
