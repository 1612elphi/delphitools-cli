# delphitools

An indie toolkit of ~40 design and publishing utilities, all in one CLI.
Offline, predictable, machine-readable. The companion to the
[delphitools](https://delphitools.info) web app.

```
$ dt p
██  #8387af
██  #718fe6
██  #5470c0
██  #ad8c27
██  #856737
```

## Install

```bash
cargo install delphitools-cli
```

Three identical binaries land in your `$PATH`: `delphi`, `delphitools`, `dt`.

Optional: install man pages so `man delphi` works:

```bash
dt install-man
```

## Quick tour

```bash
dt                                # tasting menu — 8 random tools
dt '?'                            # full command list (quote in zsh)
dt help                           # same
dt <command> --help               # per-command flags + examples
```

A few examples:

```bash
dt colour "#ff6600" hex,rgb,oklch
dt palette --strategy 80s --size 6 --seed 1
dt convert *.png --to webp --quality 80 -o webp/
dt calc "sin(pi/4) + 2^10"
dt unit "100 kg" lb oz g
dt time now --tz America/New_York
echo "hello world" | dt shavian
dt qr "https://example.com" --size 1024 -o qr.png
dt rmbg portrait.jpg                # background removal (downloads model on first use)
```

Every command supports `--json` (`-j`) for structured output, follows
`positional arg → file → stdin` for text input, and writes to
`<stem>-<op>.<ext>` for file outputs (override with `-o`).

## Tool index

- **Colour:** colour, contrast, harmony, tailwind-shades, palette, colorblind
- **Image:** crop, matte, scroll, watermark, favicon, split, convert, noise, clip, trace, svgo, rmbg
- **PDF:** preflight, zine, impose
- **Text/Type:** px2rem, rem2px, line-height, typo, wc, paper, glyph, font-info, regex, shavian
- **Generators:** qr, barcode, meta
- **Calc:** calc, base, time, unit, encode, decode, hash

Most commands have short aliases (`c` for colour, `p` for palette, `sv` for
shavian, etc.); each `--help` page lists them.

## License

[0BSD](./LICENSE). Use it however you like; no attribution required.

The runtime background-removal model (`isnet-general-use.onnx`) is downloaded
on first use of `delphi rmbg` from the [`rembg`](https://github.com/danielgatis/rembg)
project's release page and is licensed Apache 2.0.

The bundled Shavian dictionary derives from the
[delphitools](https://github.com/1612elphi/delphitools) web app.
