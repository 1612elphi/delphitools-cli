# delphitools-cli

The same [delphitools](https://delphi.tools), but in your terminal.
No network calls. No telemetry. No configs.
Everything pipes. Everything takes `-j` for JSON.
Long live the handmade web.

## install

```
cargo install delphitools-cli
```

Or grab a prebuilt binary from the [releases page](https://github.com/1612elphi/delphitools-cli/releases).

You get three names: `delphi`, `delphitools`, `dt`. Pick whichever.
Run `dt install-man` once to get `man delphi` working.

## use

```
dt                  eight random tools, fresh palette every load
dt '?'              every command
dt <cmd> --help     usage for one
dt agent            machine-readable reference for AI agents
man delphi          man page
```

## included tools

### social media

- social cropper (`crop`)
- matte genny (`matte`)
- scroll genny (`scroll`)
- watermarker (`watermark`)

### colour

- colour converter (`colour`) — hex, rgb, hsl, oklch, oklab, lab
- tailwind shade genny (`tailwind-shades`)
- harmony genny (`harmony`)
- palette genny (`palette`) — 28 strategies, all in OKLCH
- contrast checker (`contrast`)
- colour blindness simulator (`colorblind`)

### img & assets

- favicon genny (`favicon`)
- svg optimiser (`svgo`)
- image splitter (`split`)
- image converter (`convert`) — png/jpg/webp/gif/tiff/bmp/ico, with resize
- artwork enhancer (`noise`)
- background remover (`rmbg`) — Apache-licensed ISNet, model fetched on consent
- image tracer (`trace`) — raster → svg
- image clipper (`clip`)

### typo & text

- px to rem (`px2rem`, `rem2px`)
- line height calc (`line-height`)
- typo calc (`typo`) — pt, px, mm, cm, in, pc, em, rem
- paper sizes (`paper`)
- word counter (`wc`)
- glyph browser (`glyph`) — codepoint, range, name search
- font file explorer (`font-info`) — ttf/otf metadata
- regex tester (`regex`)

### print & production

- pdf preflight (`preflight`)
- zine imposer (`zine`)
- print imposer (`impose`) — saddle-stitch, perfect-bind, n-up

### other tools

- qr genny (`qr`)
- barcode genny (`barcode`)
- meta tag genny (`meta`)

### calculators

- scientific calc (`calc`)
- base converter (`base`)
- time calc (`time`)
- unit converter (`unit`)
- encoding tools (`encode`, `decode`, `hash`)

### turbo-nerd shit

- shavian transliterator (`shavian`)

## license

[0BSD](./LICENSE). Use it however you like; no attribution required.
The ISNet background-removal model and Shavian phoneme dictionary
carry their own (also-permissive) licenses — see the LICENSE file.
