//! Generate man pages at build time and embed them in the binary.
//!
//! On every build:
//!   1. Re-parse `src/cli.rs` to build the clap Command tree.
//!   2. Render `delphi.1` + one page per subcommand into `$OUT_DIR/man/`.
//!   3. Emit a `man_pages.rs` manifest that `include_bytes!`s each page,
//!      exposed as `MAN_PAGES: &[(&str, &[u8])]`.
//!
//! The runtime `delphi install-man` command iterates this manifest, so the
//! man pages always ship with the binary.

use clap::CommandFactory;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

// `src/cli.rs` is self-contained (only depends on clap + clap_complete + std),
// so we can pull it into the build script directly without involving the main
// crate. The include brings `std::path::PathBuf` into scope for us.
include!("src/cli.rs");

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=src/cli.rs");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir)?;

    let root = Cli::command().name("delphi");
    let mut entries: Vec<(String, PathBuf)> = Vec::new();

    render(&root, "delphi", &man_dir, &mut entries)?;
    for sub in root.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        let page_name = format!("delphi-{}", sub.get_name());
        render(sub, &page_name, &man_dir, &mut entries)?;
    }

    // Emit Rust manifest.
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let manifest = out_dir.join("man_pages.rs");
    let mut f = fs::File::create(&manifest)?;
    writeln!(f, "pub static MAN_PAGES: &[(&str, &[u8])] = &[")?;
    for (name, path) in &entries {
        // `include_bytes!` takes a string literal; the absolute path keeps it
        // independent of where the consuming file lives.
        writeln!(
            f,
            "    ({:?}, include_bytes!({:?})),",
            name,
            path.to_string_lossy()
        )?;
    }
    writeln!(f, "];")?;

    Ok(())
}

fn render(
    cmd: &clap::Command,
    name: &str,
    dir: &Path,
    entries: &mut Vec<(String, PathBuf)>,
) -> std::io::Result<()> {
    let man = clap_mangen::Man::new(cmd.clone())
        .title(name.to_ascii_uppercase())
        .section("1")
        .source(format!("delphitools-cli {}", env!("CARGO_PKG_VERSION")))
        .manual("delphitools manual");
    let mut buf: Vec<u8> = Vec::new();
    man.render(&mut buf)?;
    let path = dir.join(format!("{name}.1"));
    let mut f = fs::File::create(&path)?;
    f.write_all(&buf)?;
    entries.push((format!("{name}.1"), path));
    Ok(())
}
