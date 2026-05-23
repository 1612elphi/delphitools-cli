//! `delphi agent` — a compact, machine-readable reference card aimed at AI
//! agents driving the CLI. Plain text (no ANSI), stable formatting, deliberately
//! brief: agents shell-out to discover capabilities, then drill down via
//! `delphi <cmd> --help` for specifics.

use crate::error::Error;

const REFERENCE: &str = include_str!("agent.txt");

pub fn run() -> Result<(), Error> {
    print!("{REFERENCE}");
    Ok(())
}
