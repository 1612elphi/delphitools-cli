# delphitools-cli — Tool Specs

CLI equivalents of the [delphitools](https://github.com/1612elphi/delphitools) browser tools.
Tools marked **SKIP** are inherently interactive or browser-only.

---

## Social Media

### social-cropper

Crop an image to a social media aspect ratio.

```
delphi crop <image> --ratio 4:5 --position center
  -> cropped.png
```

```pseudo
img = load(image)
target = parse_ratio(ratio)  # e.g. 4:5 -> 0.8
current = img.width / img.height
if current > target:
  new_w = img.height * target
  x_off = position_offset(img.width, new_w, position)
  crop(img, x_off, 0, new_w, img.height)
else:
  new_h = img.width / target
  y_off = position_offset(img.height, new_h, position)
  crop(img, 0, y_off, img.width, new_h)
write(cropped, output)
```

### matte-generator

Place a non-square image on a square matte.

```
delphi matte <image> --ratio 1:1 --mode blur|solid|gradient [--colour #fff]
  -> matted.png
```

```pseudo
img = load(image)
side = max(img.width * ratio_w, img.height * ratio_h)
canvas = new_canvas(side * ratio_w, side * ratio_h)
if mode == "solid":
  fill(canvas, colour)
elif mode == "blur":
  draw_scaled(canvas, img, cover=true)
  gaussian_blur(canvas, radius=40)
elif mode == "gradient":
  dominant = dominant_colour(img)
  fill_gradient(canvas, lighten(dominant), darken(dominant))
draw_centered(canvas, img)
write(canvas, output)
```

### scroll-generator

Split a wide image into carousel tiles for Instagram.

```
delphi scroll <image> --ratio 4:5 --fill blur|colour [--colour #fff]
  -> tile_1.png tile_2.png ...
```

```pseudo
img = load(image)
tile_h = img.height
tile_w = tile_h * (ratio_w / ratio_h)
n_tiles = ceil(img.width / tile_w)
for i in 0..n_tiles:
  tile = new_canvas(tile_w, tile_h)
  if fill == "blur":
    draw_scaled(tile, img, cover=true)
    gaussian_blur(tile, 40)
  else:
    fill(tile, colour)
  x_src = i * tile_w
  draw(tile, img, src_x=x_src, dest_x=0)
  write(tile, "tile_{i}.png")
```

### watermarker

Composite a watermark onto an image.

```
delphi watermark <image> --mark <watermark.png> --position bottom-right --opacity 0.3 --scale 0.2
  -> watermarked.png
```

```pseudo
img = load(image)
mark = load(watermark)
mark = resize(mark, img.width * scale, img.height * scale)
mark = set_opacity(mark, opacity)
pos = compute_position(img, mark, position)  # e.g. bottom-right with padding
composite(img, mark, pos, blend_mode)
write(img, output)
```

---

## Colour

### colour-converter

Convert a colour between formats.

```
delphi colour "#ff6600" --to hsl,oklch,lab
  -> hsl(24, 100%, 50%)
  -> oklch(0.6836, 0.1955, 46.84)
  -> lab(57.58, 39.43, 62.91)
```

```pseudo
rgb = parse_any_colour(input)  # hex, rgb(), hsl(), oklch(), etc.
for fmt in formats:
  if fmt == "hex": print(rgb_to_hex(rgb))
  if fmt == "hsl": print(rgb_to_hsl(rgb))
  if fmt == "lab": print(rgb_to_xyz_d65(rgb) |> xyz_to_lab())
  if fmt == "oklch": print(rgb_to_oklab(rgb) |> oklab_to_oklch())
  # etc.
```

### tailwind-shades

Generate a Tailwind colour scale from a base colour.

```
delphi tailwind-shades "#3b82f6" --mode classic|vivid|muted|hue-shift
  -> 50: #eff6ff
  -> 100: #dbeafe
  -> ... (11 shades, 50-950)
```

```pseudo
base = parse_hex(input)
oklch = rgb_to_oklch(base)
for shade in [50, 100, 200, ..., 950]:
  t = shade / 1000  # 0.0 (lightest) to 0.95 (darkest)
  L = lerp(0.97, 0.15, t)
  C = oklch.C * chroma_curve(t, mode)  # vivid boosts mid, muted dampens
  H = oklch.H + hue_shift(t, mode)     # hue-shift rotates +-15 deg
  print(shade, oklch_to_hex(L, C, H))
```

### harmony-genny

Generate colour harmonies from a base colour.

```
delphi harmony "#ff6600" --type complementary|analogous|triadic|tetradic|split
```

```pseudo
hsl = hex_to_hsl(input)
if type == "complementary": offsets = [180]
if type == "analogous":     offsets = [-30, 30]
if type == "triadic":       offsets = [120, 240]
if type == "tetradic":      offsets = [90, 180, 270]
if type == "split":         offsets = [150, 210]
for off in offsets:
  print(hsl_to_hex(hsl.h + off, hsl.s, hsl.l))
```

### contrast-checker

Check WCAG contrast ratio between two colours.

```
delphi contrast "#000000" "#ffffff"
  -> Ratio: 21:1
  -> AA normal: PASS  AA large: PASS
  -> AAA normal: PASS  AAA large: PASS
```

```pseudo
L1 = relative_luminance(parse(fg))  # 0.2126*R + 0.7152*G + 0.0722*B (linearised)
L2 = relative_luminance(parse(bg))
ratio = (max(L1, L2) + 0.05) / (min(L1, L2) + 0.05)
print("AA normal",  ratio >= 4.5)
print("AA large",   ratio >= 3.0)
print("AAA normal", ratio >= 7.0)
print("AAA large",  ratio >= 4.5)
```

### colorblind-sim

Simulate colour blindness on an image.

```
delphi colorblind <image> --type protanopia|deuteranopia|tritanopia|achromatopsia
  -> simulated.png
```

```pseudo
img = load(image)
matrix = CVD_MATRICES[type]  # 3x3 colour transformation matrix
for each pixel (r, g, b) in img:
  [r2, g2, b2] = matrix * [r, g, b]
  set_pixel(r2, g2, b2)
write(img, output)
```

### palette-genny

Generate colour palettes using 30 strategies across 6 categories, all in OKLCH space.

```
delphi palette --strategy analogous --size 5 --format hex|css|json|png
  -> #e8d5c4 #c4a882 #9b7d5e #7a6347 #5c4a33

delphi palette --strategy 80s --size 6 --format css
  -> --palette-1: #1a1a1a;
  -> --palette-2: #ff2d95;
  -> ...

delphi palette --strategy ocean-sunset --size 4 --format json
  -> [{"hex":"#e07a5f","rgb":[224,122,95],"oklch":[0.65,0.13,38]}, ...]

delphi palette --strategy random-cohesive --size 5 --lock 0:#ff6600,3:#003366
  -> (regenerates slots 1,2,4 while keeping 0 and 3 fixed)
```

**Strategies (30):**

| Category | Strategies |
|----------|-----------|
| Random | `true-random`, `random-cohesive` |
| Colour theory | `analogous`, `complementary`, `triadic`, `split-complementary`, `tetradic`, `monochromatic` |
| Mood | `thermos`, `specimen`, `souvenir`, `curfew`, `telegraph` |
| Era | `70s`, `80s`, `90s`, `y2k` |
| Nature | `ocean-sunset`, `forest-morning`, `desert-dusk`, `arctic`, `volcanic`, `meadow` |
| Cultural | `bauhaus`, `art-deco`, `japanese`, `scandinavian`, `mexican` |

```pseudo
function generate(strategy, size, locked):
  colours = []
  for i in 0..size:
    if locked[i]: colours.append(locked[i]); continue

    if strategy == "analogous":
      base_hue = random(0, 360)
      L = random(0.4, 0.75)
      C = random(0.08, 0.2)
      H = base_hue + random(-20, 20)  # 40° spread
    elif strategy == "complementary":
      cluster = i < size/2 ? 0 : 1
      H = base_hue + cluster * 180
      L, C = random within ranges
    elif strategy == "80s":
      if random() < 0.2:
        L, C, H = 0.0, 0.0, 0  # black
      else:
        H = pick_weighted([300-340, 200-260, 270-310])  # neon pink/blue/purple
        C = random(0.18, 0.30)  # high saturation
        L = random(0.55, 0.85)
    elif strategy == "japanese":
      H = pick_weighted([
        {range: 245-270, weight: 3},  # indigo
        {range: 18-35,   weight: 2},  # vermillion
        {range: 75-95,   weight: 1},  # gold
        {range: 120-145, weight: 1},  # pine
        {range: 290-320, weight: 1},  # wisteria
        {range: 340-360, weight: 1},  # cherry blossom
      ])
      L, C = per-hue-range defaults
    # ... (each strategy defines hue ranges, weights, L/C bounds)

    colours.append(oklch_to_hex(clamp(L), clamp(C), H % 360))
  return colours

function format_output(colours, format):
  if format == "hex":  print each hex
  if format == "css":  print "--palette-{i}: {hex};"
  if format == "json": print [{hex, rgb, oklch}, ...]
  if format == "png":  render colour swatches to image
```

### palette-collection — SKIP

Static curated list of ~150 palettes. In the web app this links into palette-genny.
Could be exposed as `delphi palette --list [--category nature]` but has no generation logic.

---

## Images & Assets

### favicon-genny

Generate multi-size favicons from a source image.

```
delphi favicon <image> [--sizes 16,32,48,180,512] [--ico]
  -> favicon-16.png favicon-32.png ... [favicon.ico]
```

```pseudo
img = load(image)
img = crop_to_square(img)
for size in sizes:
  resized = resize(img, size, size, bicubic)
  write(resized, "favicon-{size}.png")
if ico:
  ico_data = encode_ico([load("favicon-16.png"), load("favicon-32.png"), ...])
  write(ico_data, "favicon.ico")
```

### svg-optimiser

Optimise and minify SVG files.

```
delphi svgo <file.svg> [-o optimised.svg]
  -> 4,231 B -> 2,108 B (50.2% reduction)
```

```pseudo
svg = read(file)
optimised = svgo.optimize(svg, {
  multipass: true,
  plugins: preset_default
})
write(optimised.data, output)
print(original_size, "->", optimised_size)
```

### image-splitter

Split an image into a grid of tiles.

```
delphi split <image> --rows 3 --cols 3
  -> tile_0_0.png tile_0_1.png ... tile_2_2.png
```

```pseudo
img = load(image)
tile_w = img.width / cols
tile_h = img.height / rows
for r in 0..rows:
  for c in 0..cols:
    tile = crop(img, c * tile_w, r * tile_h, tile_w, tile_h)
    write(tile, "tile_{r}_{c}.png")
```

### image-converter

Convert images between formats with optional resize.

```
delphi convert <image> --to webp --quality 80 [--resize 50%] [--resize 800x600]
  -> image.webp
```

```pseudo
img = load(image)
if resize:
  if percentage: img = scale(img, pct / 100)
  if dimensions: img = resize(img, w, h, lock_aspect)
encode(img, format, {
  png:  { transparency }
  jpeg: { quality, bg_colour }
  webp: { quality, lossless }
  avif: { quality }
  gif:  { max_colours, quantization }
  ico:  { sizes[], multi_size }
  icns: { sizes[], multi_size }
})
write(encoded, output)
```

### artwork-enhancer

Add colour noise overlay to artwork.

```
delphi noise <image> --opacity 0.15 --scale 1.0 [--seed 42]
  -> enhanced.png
```

```pseudo
img = load(image)
noise = generate_noise(img.width, img.height, seed)
noise = scale_noise(noise, scale)
for each pixel:
  result = overlay_blend(img_pixel, noise_pixel, opacity)
write(result, output)
```

### background-remover

Remove background from an image using ML segmentation.

```
delphi rmbg <image>
  -> image-nobg.png
```

```pseudo
img = load(image)
model = load_model("briaai/RMBG-1.4")  # ONNX segmentation model
mask = model.predict(img)               # foreground probability map
mask = resize(mask, img.width, img.height)
mask = threshold(mask, 0.5)
for each pixel:
  img.alpha = mask_value
write(img, output)
```

### image-tracer

Trace raster images to SVG vectors.

```
delphi trace <image> --preset default|detailed|posterize [--colours 4] [--blur 0]
  -> traced.svg
```

```pseudo
img = load(image)
if blur > 0: img = gaussian_blur(img, blur)
posterised = quantize_colours(img, n_colours)
for each colour_layer in posterised:
  bitmap = threshold(posterised, colour)
  paths = potrace(bitmap, {
    turnpolicy, alphamax, turdsize, opticurve
  })
  svg_paths.append(paths, colour)
svg = compose_svg(svg_paths, img.width, img.height)
write(svg, output)
```

### image-clipper

Trim transparent edges from a PNG.

```
delphi clip <image.png>
  -> image-clipped.png (was 800x600, now 720x580; trimmed 10/30/10/40 px)
```

```pseudo
img = load(image)
top = height; bottom = 0; left = width; right = 0
for each pixel (x, y):
  if alpha > 0:
    top    = min(top, y)
    bottom = max(bottom, y)
    left   = min(left, x)
    right  = max(right, x)
cropped = crop(img, left, top, right - left + 1, bottom - top + 1)
write(cropped, output)
```

### paste-image — SKIP

Clipboard-only; no file input.

---

## Typography & Text

### px-to-rem

Convert between px and rem.

```
delphi px2rem 16 [--base 16]
  -> 1rem

delphi rem2px 1.5 [--base 16]
  -> 24px
```

```pseudo
if direction == "px-to-rem": result = value / base
if direction == "rem-to-px": result = value * base
```

### line-height-calc

Calculate optimal line heights for a font size.

```
delphi line-height 16
  -> tight:    1.2  (19.2px)
  -> snug:     1.375 (22px)
  -> normal:   1.5  (24px)
  -> relaxed:  1.625 (26px)
  -> loose:    2.0  (32px)
  -> golden:   1.618 (25.9px)
```

```pseudo
for name, ratio in RATIOS:
  print(name, ratio, font_size * ratio)
```

### typo-calc

Convert between typographic units.

```
delphi typo 12pt --to px,mm,pc
  -> 16px, 4.233mm, 1pc
```

```pseudo
# normalise input to points
pt = convert_to_points(value, from_unit, base_font_size)
# convert points to target
for unit in targets:
  if unit == "px":  print(pt * (96/72))
  if unit == "mm":  print(pt * 0.3528)
  if unit == "pc":  print(pt / 12)
  if unit == "in":  print(pt / 72)
  if unit == "em":  print(pt * (96/72) / base_font_size)
  # etc.
```

### word-counter

Count words, characters, sentences, reading/speaking time.

```
delphi wc <file.txt>
  -> Words: 1,234  Characters: 6,789  Sentences: 56  Paragraphs: 12
  -> Reading: ~6 min  Speaking: ~8 min

cat essay.md | delphi wc
```

```pseudo
text = read(input)
words      = split(text, /\s+/).length
chars      = text.length
chars_ns   = remove_spaces(text).length
sentences  = split(text, /[.!?]+/).length
paragraphs = split(text, /\n\n+/).length
lines      = split(text, /\n/).length
read_time  = ceil(words / 200)
speak_time = ceil(words / 150)
```

### paper-sizes

Look up paper dimensions.

```
delphi paper a4 [--unit mm|in|pt|px --dpi 300]
  -> A4: 210 × 297 mm (2480 × 3508 px @300dpi)

delphi paper --series a|b|c|us
  -> A0: 841 × 1189 mm
  -> A1: 594 × 841 mm
  -> ...
```

```pseudo
sizes = {
  "a4": {w: 210, h: 297, unit: "mm"},
  "letter": {w: 8.5, h: 11, unit: "in"},
  # ...ISO A/B/C series, US sizes
}
s = sizes[name]
if unit != s.unit: convert(s, unit)
if dpi: print(s.w * dpi / 25.4, s.h * dpi / 25.4, "px")
print(s.w, s.h, unit)
```

### glyph-browser

Look up Unicode characters by codepoint, name, or range.

```
delphi glyph U+2603
  -> ☃  U+2603  html: &#x2603;  css: \002603  js: \u2603

delphi glyph --range arrows
  -> ← U+2190  → U+2192  ↑ U+2191  ↓ U+2193 ...

delphi glyph --search "snowman"
  -> ☃ U+2603  ⛄ U+26C4  ⛇ U+26C7
```

```pseudo
if codepoint:
  char = String.fromCodePoint(parse_hex(input))
  print(char, "U+" + hex, "html: &#x" + hex + ";",
        "css: \\" + hex, "js: " + js_escape(codepoint))
elif range:
  [start, end] = RANGES[name]  # e.g. arrows = [0x2190, 0x21ff]
  for cp in start..end:
    print(char(cp), "U+" + hex(cp))
elif search:
  for cp in UNICODE_DATABASE:
    if name_of(cp).contains(query):
      print(char(cp), "U+" + hex(cp))
```

### font-explorer

Extract metadata and preview info from font files.

```
delphi font-info <font.ttf>
  -> Name: Inter Regular
  -> Format: TrueType (.ttf)
  -> PostScript: Inter-Regular
  -> CSS: @font-face { font-family: "Inter"; src: url("font.ttf") format("truetype"); }
```

```pseudo
ext = file_extension(path)
format = {ttf: "truetype", otf: "opentype", woff: "woff", woff2: "woff2"}[ext]
name = filename_without_ext(path)
postscript = name.replace(" ", "-")
print("Name:", name)
print("Format:", format)
print("PostScript:", postscript)
print("CSS:", generate_font_face(name, path, format))
```

---

## Print & Production

### pdf-preflight

Analyse a PDF for print-readiness issues.

```
delphi preflight <file.pdf>
  -> Version: 1.7  Pages: 12  Encrypted: No
  -> Page 1: 210×297mm (A4)  TrimBox: yes  BleedBox: 3mm
  -> Fonts: 3 embedded, 1 NOT EMBEDDED (Arial)
  -> Warnings: unembedded font, missing bleed on page 4
```

```pseudo
pdf = parse_pdf(file)
print(pdf.version, pdf.page_count, pdf.encrypted)
for page in pdf.pages:
  media = page.MediaBox   # required
  trim  = page.TrimBox    # optional
  bleed = page.BleedBox   # optional
  print(page_dimensions(media))
  if not trim: warn("no TrimBox")
  if not bleed: warn("no BleedBox")
fonts = extract_fonts(pdf)
for font in fonts:
  if not font.embedded: warn("unembedded: " + font.name)
check_transparency(pdf)
check_images(pdf)
```

### zine-imposer

Impose images into a single-sheet folded-zine layout. Two fold templates:

- **mini8** (default) — classic 8-page mini-zine on a 4×2 grid, single-sided,
  with a central fold-and-cut slit. Requires exactly 8 images.
- **accordion** — zig-zag concertina on a 1×N grid (N ∈ {4,6,8}), no cut.
  Single-sided is a fold-out strip (N images). `--double` makes a continuous
  two-sided booklet (2×N images: front 1..N, back N+1..2N), printed flip-on-short-edge.
  `--split` (two-up) stacks two identical copies of the strip per sheet (rows = 2,
  half-height panels); cut the sheet in half horizontally for two copies. The image
  count is unchanged by `--split` because the lanes are copies, not new content.
  `--panels`, `--double`, and `--split` are accordion-only and ignored for mini8.

```
# Classic 8-page mini-zine (8 images)
delphi zine <img1> ... <img8> --dpi 300 --paper a4
  -> zine.pdf

# 6-panel accordion, single-sided (6 images)
delphi zine --fold accordion --panels 6 <img1> ... <img6>

# 8-panel accordion, double-sided (16 images -> 2-page PDF)
delphi zine --fold accordion --panels 8 --double <img1> ... <img16>

# 8-panel accordion, two-up split (still 8 images -> 1 page, two copies on the sheet)
delphi zine --fold accordion --panels 8 --split <img1> ... <img8>
```

```pseudo
required = fold == mini8 ? 8 : (double ? 2*panels : panels)  # unchanged by --split
imgs = load_all(images)  # count must == required
sheet_w, sheet_h = landscape(paper_size(paper))   # long edge horizontal
cols, rows = fold == mini8 ? (4, 2) : (panels, split ? 2 : 1)
cell_w, cell_h = sheet_w / cols, sheet_h / rows

# mini8: fixed fold-and-cut order, top row rotated 180
#   [p5↻ p4↻ p3↻ p2↻ / p6 p7 p8 p1]
# accordion: page (c+1) at col c upright; double-sided adds a back page
#   (pages N+1..2N at col c upright). --split duplicates each side's
#   placements into both lanes (row 0 and row 1 carry the same pages).
for side in fold.sides():            # 1 page, or 2 for accordion --double
  for {page, col, row, rotation} in side:
    y = (rows - 1 - row) * cell_h    # printpdf origin is bottom-left
    draw(page_img, imgs[page-1], x=col*cell_w, y, rotate=rotation)
write_pdf(sides, output)
```

### imposer

Full PDF imposition for booklet/saddle-stitch/n-up printing.

```
delphi impose <file.pdf> --layout saddle-stitch|perfect-bind|4up|n-up
    [--pages-per-sheet 4] [--margins 10mm] [--gutter 5mm] [--creep 0.5mm]
    [--crop-marks] [--duplex]
  -> imposed.pdf
```

```pseudo
pdf = parse_pdf(file)
pages = pdf.pages
if layout == "saddle-stitch":
  # pair pages: (last, first), (second, second-last), ...
  signatures = saddle_stitch_order(pages)
elif layout == "perfect-bind":
  signatures = perfect_bind_order(pages, pages_per_signature)
elif layout == "n-up":
  signatures = chunk(pages, n)

for sig in signatures:
  sheet = new_page(paper_size)
  for i, page in enumerate(sig):
    slot = compute_slot(i, n_up, margins, gutter)
    if creep > 0: slot.x += creep_offset(sig_index)
    place(sheet, page, slot, scale_mode)
  if crop_marks: draw_crop_marks(sheet, slots)
  if duplex: add_back_sheet(sheet, sig)
write_pdf(sheets, output)
```

---

## Other Tools

### qr-genny

Generate styled QR codes.

```
delphi qr "https://example.com" --size 1024 --fg "#000" --bg "#fff"
    [--logo logo.png] [--dot-style rounded] [--error-level H]
  -> qr.png
```

```pseudo
data = input_text
matrix = qr_encode(data, error_level)  # L/M/Q/H
canvas = new_canvas(size, size)
fill(canvas, bg)
module_size = size / (matrix.size + padding * 2)
for r, c in matrix:
  if matrix[r][c]:
    draw_module(canvas, r, c, module_size, dot_style, fg)
draw_corners(canvas, corner_style, fg)
if logo:
  logo_img = load(logo)
  draw_centered(canvas, logo_img, max_size=size * 0.25)
write(canvas, output)
```

### code-genny

Generate 1D/2D barcodes.

```
delphi barcode "1234567890128" --format ean13|code128|datamatrix|aztec|pdf417
  -> barcode.png
```

```pseudo
validate(data, format)  # character set, length, check digit
encoded = bwip_js.encode(data, format, {
  height, width, scale, includetext
})
write(encoded, output)
```

### meta-tag-genny

Generate HTML meta tags.

```
delphi meta --title "My Page" --description "A description" --url "https://..."
    [--image "https://..."] [--type website|article]
```

```pseudo
print('<meta charset="UTF-8">')
print('<title>' + title + '</title>')
print('<meta name="description" content="' + description + '">')
print('<meta property="og:title" content="' + title + '">')
print('<meta property="og:description" content="' + description + '">')
print('<meta property="og:url" content="' + url + '">')
if image: print('<meta property="og:image" content="' + image + '">')
print('<meta name="twitter:card" content="summary_large_image">')
# ... etc
```

### regex-tester

Test a regex pattern against input text.

```
delphi regex '\d{3}-\d{4}' "Call 555-1234 or 555-5678" --flags g
  -> Match 1: "555-1234" (pos 5-13)
  -> Match 2: "555-5678" (pos 17-25)
```

```pseudo
re = compile(pattern, flags)
matches = re.find_all(text)
for m in matches:
  print(m.index, m.value)
  for g in m.groups:
    print("  group", g.index, g.value)
```

### markdown-writer — SKIP

Interactive text editor.

### tailwind-cheatsheet — SKIP

Static reference; no computation.

---

## Calculators

### sci-calc

Evaluate mathematical expressions.

```
delphi calc "sin(pi/4) + 2^10"
  -> 1024.7071067811865
```

```pseudo
result = mathjs.evaluate(expression)
# supports: sin cos tan asin acos atan log ln sqrt abs
#           ^ ** % ! pi e
#           degree/radian mode
print(result)
```

### base-converter

Convert numbers between bases.

```
delphi base 255 --from dec --to hex,bin,oct
  -> hex: FF
  -> bin: 11111111
  -> oct: 377

delphi base FF --from hex --bitwise "AND 0F"
  -> 0F (15)
```

```pseudo
n = parse_int(value, from_base)
for base in targets:
  print(base, to_string(n, base))
if bitwise:
  op, operand = parse(bitwise)
  b = parse_int(operand, from_base)
  if op == "AND":  print(n & b)
  if op == "OR":   print(n | b)
  if op == "XOR":  print(n ^ b)
  if op == "NOT":  print(~n)
  if op == "SHL":  print(n << b)
  if op == "SHR":  print(n >> b)
```

### time-calc

Unix timestamp and date arithmetic.

```
delphi time now
  -> 2026-04-09T14:30:00Z  (1744209000)

delphi time 1744209000 --to iso,rfc2822 [--tz America/New_York]

delphi time "2026-04-09" --add 30d
  -> 2026-05-09T00:00:00Z
```

```pseudo
if input == "now": t = current_time()
elif is_number(input): t = from_unix(input)
else: t = parse_date(input)

if tz: t = convert_timezone(t, tz)
if add: t = t + parse_duration(add)

for fmt in formats:
  print(format(t, fmt))
print("unix:", to_unix(t))
```

### unit-converter

Convert between units.

```
delphi unit 100 kg --to lb,oz,g
  -> 220.462 lb
  -> 3527.396 oz
  -> 100000 g
```

```pseudo
# categories: length, weight, data, temperature, volume, speed, pressure
base = to_base_unit(value, from_unit)  # e.g. kg -> grams
for unit in targets:
  print(from_base_unit(base, unit))
# temperature special-cased: C/F/K with direct formulas
```

### encoder

Base64, URL encoding, and hash generation.

```
delphi encode base64 "Hello, World!"
  -> SGVsbG8sIFdvcmxkIQ==

delphi decode base64 "SGVsbG8sIFdvcmxkIQ=="
  -> Hello, World!

delphi hash sha256 "Hello, World!"
  -> dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f
```

```pseudo
if mode == "base64_encode": print(btoa(utf8_encode(input)))
if mode == "base64_decode": print(utf8_decode(atob(input)))
if mode == "url_encode":    print(percent_encode(input))
if mode == "url_decode":    print(percent_decode(input))
if mode == "hash":          print(crypto_hash(algorithm, utf8_encode(input)))
```

---

## Turbo-nerd Shit

### shavian-transliterator

Transliterate English text to the Shavian alphabet.

```
delphi shavian "The quick brown fox"
  -> 𐑞 𐑒𐑢𐑦𐑒 𐑚𐑮𐑬𐑯 𐑓𐑪𐑒𐑕
```

```pseudo
dict = load_phoneme_dictionary()  # word -> shavian mapping
for word in split(input):
  if word in dict:
    output.append(dict[word])
  else:
    output.append(phonetic_fallback(word))  # rule-based approximation
print(join(output, " "))
```
