use std::io::IsTerminal;

pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Render a 24-bit ANSI colour block (two spaces with background colour).
/// Returns empty string when stdout is not a TTY.
pub fn colour_block(r: u8, g: u8, b: u8) -> String {
    if is_tty() {
        format!("\x1b[48;2;{r};{g};{b}m  \x1b[0m")
    } else {
        String::new()
    }
}
