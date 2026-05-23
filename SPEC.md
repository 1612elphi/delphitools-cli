# delphitools-cli — Specification

CLI companion to [delphitools](https://github.com/1612elphi/delphitools). Same tools, same philosophy: privacy-first, no network calls, no accounts. Everything runs locally.

See [TOOLS.md](./TOOLS.md) for per-tool specifications and pseudocode.

---

## Principles

1. **Offline by default.** No tool should require a network connection. Any future tool that does must be opt-in and clearly marked.
2. **Pipeable.** Every tool that accepts text input must read from stdin when no file/argument is given. Every tool that produces text output must write to stdout.
3. **Predictable output.** Human-readable by default, machine-readable with `--json`. No colour codes, spinners, or progress bars when stdout is not a TTY.
4. **No config files.** No dotfiles, no init step, no global state. Flags and arguments are the interface.
5. **British spelling.** `colour`, `optimiser`, etc. — consistent with the web version.

---

## Invocation

```
delphi <command> [arguments] [flags]
```

### Global flags

| Flag | Short | Description |
|------|-------|-------------|
| `--help` | `-h` | Show help for a command |
| `--version` | `-v` | Print version and exit |
| `--json` | `-j` | Output structured JSON instead of human-readable text |
| `--output <path>` | `-o` | Write output to a file instead of stdout (for file-producing commands) |
| `--quiet` | `-q` | Suppress informational output; only emit the result |

### Commands

Commands map 1:1 to tools. The full list is in TOOLS.md. Short aliases are provided where natural:

| Command | Alias | Category |
|---------|-------|----------|
| `colour` | `col` | Colour |
| `contrast` | | Colour |
| `tailwind-shades` | `tw` | Colour |
| `harmony` | | Colour |
| `palette` | `pal` | Colour |
| `colorblind` | `cb` | Colour |
| `crop` | | Social Media |
| `matte` | | Social Media |
| `scroll` | | Social Media |
| `watermark` | `wm` | Social Media |
| `favicon` | `fav` | Images & Assets |
| `svgo` | | Images & Assets |
| `split` | | Images & Assets |
| `convert` | `conv` | Images & Assets |
| `noise` | | Images & Assets |
| `rmbg` | | Images & Assets |
| `trace` | | Images & Assets |
| `clip` | | Images & Assets |
| `px2rem` | | Typography & Text |
| `rem2px` | | Typography & Text |
| `line-height` | `lh` | Typography & Text |
| `typo` | | Typography & Text |
| `wc` | | Typography & Text |
| `paper` | | Typography & Text |
| `glyph` | | Typography & Text |
| `font-info` | | Typography & Text |
| `preflight` | | Print & Production |
| `zine` | | Print & Production |
| `impose` | | Print & Production |
| `qr` | | Other Tools |
| `barcode` | `bc` | Other Tools |
| `meta` | | Other Tools |
| `regex` | `re` | Other Tools |
| `calc` | | Calculators |
| `base` | | Calculators |
| `time` | | Calculators |
| `unit` | | Calculators |
| `encode` | `enc` | Calculators |
| `decode` | `dec` | Calculators |
| `hash` | | Calculators |
| `shavian` | `shaw` | Turbo-nerd |

---

## I/O Conventions

### Text input

Tools that accept text (e.g. `wc`, `regex`, `shavian`, `calc`, `encode`) follow this precedence:

1. Positional argument: `delphi calc "2 + 2"`
2. File argument: `delphi wc notes.txt`
3. Stdin: `echo "hello" | delphi shavian`

If none are provided and stdin is a TTY, print usage and exit with code 1.

### File input

Tools that accept images or files (e.g. `crop`, `convert`, `rmbg`) take a positional path argument. Glob expansion is left to the shell. Multiple files are processed sequentially when a tool supports batch input.

```
delphi convert *.png --to webp
```

### File output

By default, file-producing tools write to the current directory using a predictable naming scheme:

```
<original-stem>-<operation>.<ext>

photo.png  ->  photo-cropped.png
logo.svg   ->  logo-optimised.svg
data.pdf   ->  data-imposed.pdf
```

Override with `--output` / `-o`:

```
delphi crop photo.png --ratio 1:1 -o square.png
```

When processing multiple files, `-o` must be a directory:

```
delphi convert *.png --to webp -o converted/
```

### Text output

Human-readable by default. When `--json` is passed, output is a single JSON object (or array) on stdout. Tools that produce both a result and metadata (e.g. `svgo` reporting size reduction) structure it as:

```json
{
  "result": "optimised.svg",
  "original_size": 4231,
  "optimised_size": 2108,
  "reduction": 0.502
}
```

### Colour output

When stdout is a TTY:
- Colour swatches are rendered as ANSI 24-bit colour blocks (e.g. `  #ff6600`)
- Diagnostic messages use colour (errors red, warnings yellow)
- Progress indicators (spinners, bars) are allowed for long operations like `rmbg`

When stdout is not a TTY (piped or redirected):
- No ANSI escape codes
- No progress indicators
- Plain text only

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Usage error (bad arguments, missing input) |
| `2` | Input error (file not found, corrupt image, invalid colour) |
| `3` | Processing error (operation failed) |

Errors are written to stderr. The result (if any) still goes to stdout.

---

## Cross-Platform

### Supported platforms

- macOS (arm64, x86_64)
- Linux (x86_64, arm64)
- Windows (x86_64, arm64)

### Distribution

Ship as a single self-contained binary. No runtime dependencies for the user to install. The binary should include or bundle any required data files (Unicode tables, colour name databases, phoneme dictionaries, ML models, etc.).

Exception: the `rmbg` (background remover) tool may download its ML model on first use due to size. If so, it must:
- Clearly tell the user what it's downloading and from where
- Cache the model locally (platform-appropriate cache dir)
- Work offline after the first download
- Fail gracefully with a clear message if offline and the model isn't cached

### File paths

- Accept both forward slashes and backslashes on Windows
- Handle Unicode filenames
- Use platform-appropriate temp directories for intermediate files

### Shell integration

The binary should work in any shell (bash, zsh, fish, PowerShell, cmd.exe). No shell-specific features should be required for basic operation. Tab completion scripts can be generated:

```
delphi --completions bash > ~/.local/share/bash-completion/completions/delphi
delphi --completions zsh > ~/.zfunc/_delphi
delphi --completions fish > ~/.config/fish/completions/delphi.fish
delphi --completions powershell > delphi.ps1
```

---

## Batch Processing

Any file-producing tool can process multiple inputs:

```
delphi convert a.png b.png c.png --to webp
delphi crop *.jpg --ratio 16:9
```

Behaviour:
- Files are processed sequentially
- Each file's result is reported as it completes
- A failure on one file does not abort the rest
- Exit code is `0` only if all files succeed; otherwise `3`
- With `--json`, output is an array of result objects

---

## Colour Input

All tools that accept colours should parse any of these formats:

| Format | Example |
|--------|---------|
| Hex (3, 4, 6, 8 digit) | `#f60`, `#ff6600`, `#ff660080` |
| RGB | `rgb(255, 102, 0)`, `255 102 0` |
| HSL | `hsl(24, 100%, 50%)` |
| OKLCH | `oklch(0.68 0.20 47)` |
| OKLab | `oklab(0.68 0.10 0.15)` |
| Named | `rebeccapurple`, `tomato` |

Bare hex values without `#` are accepted when unambiguous (e.g. `ff6600`).

---

## Help & Discovery

```
delphi                  # List all commands grouped by category
delphi --help           # Same, with global flags documented
delphi <command> --help # Command-specific usage, flags, and examples
```

Help text should include at least one realistic example per command.

---

## Versioning

Follow [SemVer](https://semver.org/). The version string includes the build date:

```
delphi --version
delphitools-cli 0.1.0 (2026-04-09)
```

---

## Non-Goals

- **No TUI.** No full-screen interfaces, no interactive modes, no curses. The CLI reads arguments, does work, and exits.
- **No config files.** No `.delphirc`, no `~/.config/delphi/`. Defaults are sensible; overrides are flags.
- **No plugins.** The tool set is fixed per release. New tools come with new versions.
- **No network features.** No update checks, no telemetry, no analytics, no phoning home. Ever.
- **No watch mode.** Use your shell's file-watching tools if you need that.
