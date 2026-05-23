//! Install the bundled man pages onto the user's filesystem.
//!
//! The pages are generated at build time from the clap CLI definition and
//! baked into the binary (see `build.rs`). The `MAN_PAGES` slice holds
//! `(filename, content)` pairs we write out on demand.

use crate::error::Error;
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/man_pages.rs"));

pub fn run(dir: Option<&Path>, dry_run: bool) -> Result<(), Error> {
    let target = match dir {
        Some(d) => d.to_path_buf(),
        None => default_dir()?,
    };

    if !dry_run {
        std::fs::create_dir_all(&target)
            .map_err(|e| Error::Processing(format!("could not create {}: {e}", target.display())))?;
    }

    let mut wrote = 0usize;
    for (name, content) in MAN_PAGES {
        let path = target.join(name);
        if dry_run {
            println!("{}", path.display());
        } else {
            std::fs::write(&path, content)
                .map_err(|e| Error::Processing(format!("could not write {}: {e}", path.display())))?;
        }
        wrote += 1;
    }

    if !dry_run {
        println!(
            "installed {wrote} man page(s) to {}",
            target.display()
        );
        if dir.is_none() {
            // Hint about MANPATH if the default dir isn't on it.
            if !on_manpath(&target) {
                eprintln!();
                eprintln!(
                    "note: {} may not be on your MANPATH. Add it to your shell rc:",
                    target.parent().unwrap_or(&target).display()
                );
                eprintln!(
                    "  export MANPATH=\"{}:$MANPATH\"",
                    target.parent().unwrap_or(&target).display()
                );
            }
        }
    }

    Ok(())
}

fn default_dir() -> Result<PathBuf, Error> {
    let home = std::env::var_os("HOME")
        .ok_or_else(|| Error::Processing("HOME is not set; pass --dir to choose a location".into()))?;
    Ok(PathBuf::from(home).join(".local/share/man/man1"))
}

/// Quick heuristic: is the man-page root (`…/share/man`) on MANPATH?
/// `dir` is the man1 subdirectory, so the root is its immediate parent.
fn on_manpath(dir: &Path) -> bool {
    let root = match dir.parent() {
        Some(p) => p,
        None => return false,
    };
    let root_str = root.display().to_string();
    if let Ok(mp) = std::env::var("MANPATH") {
        if mp.split(':').any(|p| p == root_str) {
            return true;
        }
    }
    if let Ok(out) = std::process::Command::new("manpath").output() {
        let s = String::from_utf8_lossy(&out.stdout);
        if s.trim().split(':').any(|p| p == root_str) {
            return true;
        }
    }
    false
}
