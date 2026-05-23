//! Generate man pages from the clap CLI definition.
//!
//! Usage:
//!   cargo run --example gen-man -- [out-dir]
//!
//! Writes `delphi.1` plus `delphi-<subcommand>.1` for every subcommand.
//! Out-dir defaults to `man/` next to Cargo.toml. The pages are roff text;
//! preview with `mandoc man/delphi.1` (macOS) or `man -l man/delphi.1` (Linux).
//!
//! To install system-wide so `man delphi` works:
//!   install -d ~/.local/share/man/man1
//!   install -m 644 man/*.1 ~/.local/share/man/man1/
//! Then add `~/.local/share/man` to MANPATH if it isn't already.

use clap::CommandFactory;
use delphitools_cli::cli::Cli;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() -> std::io::Result<()> {
    let outdir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("man"));
    std::fs::create_dir_all(&outdir)?;

    let root = Cli::command().name("delphi");
    let mut count = 0;

    // Top-level page
    write_man(&root, "delphi", &outdir)?;
    count += 1;

    // One page per subcommand: delphi-palette.1, delphi-shavian.1, …
    for sub in root.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        let sub_name = sub.get_name();
        let page_name = format!("delphi-{sub_name}");
        // clap_mangen renders the subcommand standalone. We can't easily
        // rename the inner Command (`Str` wants `'static`), so the synopsis
        // will say `<sub>` while the filename follows the conventional
        // `delphi-<sub>.1` form.
        write_man(sub, &page_name, &outdir)?;
        count += 1;
    }

    eprintln!("wrote {count} man page(s) to {}/", outdir.display());
    eprintln!();
    eprintln!("to install for `man delphi`:");
    eprintln!("  install -d ~/.local/share/man/man1");
    eprintln!(
        "  install -m 644 {}/*.1 ~/.local/share/man/man1/",
        outdir.display()
    );
    Ok(())
}

fn write_man(cmd: &clap::Command, name: &str, outdir: &Path) -> std::io::Result<()> {
    let man = clap_mangen::Man::new(cmd.clone())
        .title(name.to_ascii_uppercase())
        .section("1")
        .source(format!("delphitools-cli {}", env!("CARGO_PKG_VERSION")))
        .manual("delphitools manual");

    let mut buf: Vec<u8> = Vec::new();
    man.render(&mut buf)?;

    let path = outdir.join(format!("{name}.1"));
    let mut f = std::fs::File::create(&path)?;
    f.write_all(&buf)?;
    Ok(())
}
