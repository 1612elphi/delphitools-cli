use crate::colour::Colour;
use crate::error::Error;
use serde_json::json;

const ALL_FORMATS: &[&str] = &["hex", "rgb", "hsl", "oklch", "oklab", "lab"];

pub fn run(input: &str, formats: &[String], as_json: bool, pretty: bool) -> Result<(), Error> {
    let colour = Colour::parse(input)?;
    let fmts: Vec<&str> = if formats.is_empty() {
        ALL_FORMATS.to_vec()
    } else {
        formats.iter().map(|s| s.as_str()).collect()
    };

    if as_json {
        let mut map = serde_json::Map::new();
        for fmt in &fmts {
            map.insert(fmt.to_string(), json!(colour.format_as(fmt)?));
        }
        println!("{}", serde_json::to_string_pretty(&map).unwrap());
    } else if pretty {
        print_pretty(colour, &fmts)?;
    } else if fmts.len() == 1 {
        println!("{}", colour.format_as(fmts[0])?);
    } else {
        for fmt in &fmts {
            println!("{}: {}", fmt, colour.format_as(fmt)?);
        }
    }
    Ok(())
}

fn print_pretty(colour: Colour, fmts: &[&str]) -> Result<(), Error> {
    let (r, g, b) = colour.to_u8();
    let bg = format!("\x1b[48;2;{r};{g};{b}m");
    let reset = "\x1b[0m";

    let mut lines: Vec<String> = Vec::new();
    for fmt in fmts {
        lines.push(colour.format_as(fmt)?);
    }

    let inner_w = lines.iter().map(|l| l.len()).max().unwrap_or(0).max(20);
    let swatch_w = inner_w;
    let pad = inner_w + 4; // 2 margin each side

    println!("╭{}╮", "─".repeat(pad));
    println!("│  {}{}{}{reset}  │", bg, " ".repeat(swatch_w), reset);
    println!("│  {}{}{}{reset}  │", bg, " ".repeat(swatch_w), reset);
    println!("│{}│", " ".repeat(pad));
    for line in &lines {
        println!("│  {:width$}  │", line, width = inner_w);
    }
    println!("╰{}╯", "─".repeat(pad));

    Ok(())
}
