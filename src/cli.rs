use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "delphi",
    version,
    about = "delphitools — indie toolkit",
    after_help = "Run `delphi <command> --help` for command-specific usage, or `delphi ?` for the full list."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output structured JSON instead of human-readable text
    #[arg(long, short = 'j', global = true)]
    pub json: bool,

    /// Suppress informational output; only emit the result
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Write output to a file instead of stdout (single-file tools) or a directory (batch tools)
    #[arg(long, short, global = true)]
    pub output: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Command {
    // ── Colour ──────────────────────────────────────────────────────────────
    /// Convert a colour between formats
    #[command(visible_aliases = ["col", "c"])]
    Colour {
        /// Colour to convert (hex, rgb, hsl, oklch, oklab, or name)
        colour: String,

        /// Target formats (hex, rgb, hsl, oklch, oklab, lab). Shows all if omitted.
        formats: Vec<String>,

        /// Show a pretty colour card
        #[arg(long, short)]
        pretty: bool,
    },

    /// Generate colour harmonies from a base colour
    #[command(visible_aliases = ["harm", "h"])]
    Harmony {
        colour: String,
        harmony_type: Option<String>,
        #[arg(long, short)]
        pretty: bool,
    },

    /// Check WCAG contrast ratio between two colours
    #[command(visible_alias = "contr")]
    Contrast {
        fg: String,
        bg: String,
    },

    /// Generate Tailwind CSS colour shades from a base colour
    #[command(visible_aliases = ["tw", "shades"])]
    TailwindShades {
        colour: String,
        mode: Option<String>,
        #[arg(long, short)]
        pretty: bool,
    },

    /// Generate colour palettes using 28 strategies across 6 categories
    #[command(visible_aliases = ["pal", "p"])]
    Palette {
        /// Strategy name (e.g. analogous, 80s, ocean-sunset). Omit to list strategies.
        #[arg(long)]
        strategy: Option<String>,

        /// Number of colours (default 5)
        #[arg(long, default_value = "5")]
        size: usize,

        /// Output format: hex (default), css, json, png
        #[arg(long, default_value = "hex")]
        format: String,

        /// Lock specific slots, comma-separated: "0:#ff6600,3:#003366"
        #[arg(long)]
        lock: Option<String>,

        /// Seed for reproducible output (any 64-bit unsigned integer)
        #[arg(long)]
        seed: Option<u64>,

        /// Prefix each colour with its slot index (e.g. `[0]`) — useful for `--lock`
        #[arg(long, short)]
        pretty: bool,

        /// List all available strategies and exit
        #[arg(long)]
        list: bool,
    },

    /// Simulate colour blindness on an image or a colour
    #[command(visible_aliases = ["cb", "cvd"])]
    Colorblind {
        /// Input image file, or a colour string when used with --colour
        input: Option<String>,

        /// Type: normal, protanopia, deuteranopia, tritanopia, protanomaly,
        /// deuteranomaly, tritanomaly, achromatopsia, achromatomaly
        #[arg(long, default_value = "deuteranopia")]
        cb_type: String,

        /// Treat the input as a colour string rather than an image path
        #[arg(long)]
        colour: bool,
    },

    // ── Social Media ────────────────────────────────────────────────────────
    /// Crop an image to a social media aspect ratio
    #[command(visible_alias = "cr")]
    Crop {
        /// Input image(s)
        images: Vec<PathBuf>,

        /// Aspect ratio (e.g. 1:1, 4:5, 16:9)
        #[arg(long, default_value = "1:1")]
        ratio: String,

        /// Crop position: center, top, bottom, left, right, top-left, top-right, bottom-left, bottom-right
        #[arg(long, default_value = "center")]
        position: String,
    },

    /// Place a non-square image on a square (or aspect-ratio) matte
    #[command(visible_alias = "m")]
    Matte {
        images: Vec<PathBuf>,

        #[arg(long, default_value = "1:1")]
        ratio: String,

        /// Matte fill mode: solid, blur, gradient
        #[arg(long, default_value = "blur")]
        mode: String,

        /// Background colour (for --mode solid)
        #[arg(long, default_value = "#ffffff")]
        colour: String,
    },

    /// Split a wide image into carousel tiles for Instagram
    #[command(visible_alias = "scr")]
    Scroll {
        image: PathBuf,

        #[arg(long, default_value = "4:5")]
        ratio: String,

        /// Fill mode for letterbox space: blur, colour
        #[arg(long, default_value = "blur")]
        fill: String,

        #[arg(long, default_value = "#ffffff")]
        colour: String,
    },

    /// Composite a watermark onto an image
    #[command(visible_aliases = ["wm", "mark"])]
    Watermark {
        images: Vec<PathBuf>,

        /// Watermark image
        #[arg(long)]
        mark: PathBuf,

        /// Position: top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right
        #[arg(long, default_value = "bottom-right")]
        position: String,

        /// Opacity 0.0 – 1.0
        #[arg(long, default_value = "0.3")]
        opacity: f32,

        /// Watermark scale relative to the longest edge of the input image
        #[arg(long, default_value = "0.2")]
        scale: f32,
    },

    // ── Images & Assets ─────────────────────────────────────────────────────
    /// Generate multi-size favicons from a source image
    #[command(visible_aliases = ["fav", "f"])]
    Favicon {
        image: PathBuf,

        /// Comma-separated sizes (default: 16,32,48,180,512)
        #[arg(long, default_value = "16,32,48,180,512")]
        sizes: String,

        /// Also emit a multi-size favicon.ico
        #[arg(long)]
        ico: bool,
    },

    /// Optimise SVG files
    #[command(visible_alias = "svg")]
    Svgo {
        files: Vec<PathBuf>,
    },

    /// Split an image into a grid of tiles
    #[command(visible_alias = "sp")]
    Split {
        image: PathBuf,

        #[arg(long, default_value = "1")]
        rows: u32,

        #[arg(long, default_value = "1")]
        cols: u32,
    },

    /// Convert images between formats (with optional resize)
    #[command(visible_aliases = ["conv", "cv"])]
    Convert {
        images: Vec<PathBuf>,

        /// Target format: png, jpeg, jpg, webp, gif, tiff, bmp, ico
        #[arg(long)]
        to: String,

        /// JPEG/WebP quality (1–100, default 85)
        #[arg(long, default_value = "85")]
        quality: u8,

        /// Resize: "WxH", "Wx", "xH", or "P%" (e.g. 800x600, 50%)
        #[arg(long)]
        resize: Option<String>,
    },

    /// Add colour noise overlay to artwork
    #[command(visible_alias = "grain")]
    Noise {
        images: Vec<PathBuf>,

        /// Opacity 0.0 – 1.0
        #[arg(long, default_value = "0.15")]
        opacity: f32,

        /// Noise scale (1.0 = 1px grain)
        #[arg(long, default_value = "1.0")]
        scale: f32,

        /// Random seed (default: random)
        #[arg(long)]
        seed: Option<u64>,
    },

    /// Remove background from an image (downloads a ~170 MB Apache-licensed ONNX model on first use)
    #[command(visible_alias = "nobg")]
    Rmbg {
        images: Vec<PathBuf>,

        /// Pre-approve the one-time model download (required in non-interactive mode)
        #[arg(long)]
        approve: bool,
    },

    /// Trace raster images to SVG vectors
    #[command(visible_alias = "vec")]
    Trace {
        image: PathBuf,

        /// Preset: default, detailed, posterize
        #[arg(long, default_value = "default")]
        preset: String,

        /// Number of colours (overrides preset)
        #[arg(long)]
        colours: Option<u32>,

        /// Pre-blur radius
        #[arg(long, default_value = "0")]
        blur: f32,
    },

    /// Trim transparent edges from a PNG
    #[command(visible_alias = "trim")]
    Clip {
        images: Vec<PathBuf>,
    },

    // ── Typography & Text ───────────────────────────────────────────────────
    /// Convert px to rem
    #[command(visible_alias = "pr")]
    Px2rem {
        value: f64,
        #[arg(long, default_value = "16")]
        base: f64,
    },

    /// Convert rem to px
    #[command(visible_alias = "rp")]
    Rem2px {
        value: f64,
        #[arg(long, default_value = "16")]
        base: f64,
    },

    /// Compute line-height values for a given font size
    #[command(visible_aliases = ["lh", "lineh"])]
    LineHeight {
        /// Font size in pixels (defaults to 16)
        #[arg(default_value = "16")]
        font_size: f64,
        /// Show only a single named ratio (tight, snug, normal, relaxed, loose, golden)
        name: Option<String>,
    },

    /// Convert between typographic units (pt, px, mm, em, rem, pc, in, cm)
    #[command(visible_alias = "type")]
    Typo {
        /// Value with unit, e.g. "12pt"
        value: String,

        /// Target unit(s) (e.g. px,mm,pc). Shows all if omitted.
        targets: Vec<String>,

        /// Base font size in px (for em/rem conversions)
        #[arg(long, default_value = "16")]
        base: f64,
    },

    /// Count words, characters, sentences, and reading time
    #[command(visible_aliases = ["words", "w"])]
    Wc {
        input: Option<String>,
    },

    /// Look up paper size dimensions
    #[command(visible_alias = "page")]
    Paper {
        name: Option<String>,
        #[arg(long)]
        series: Option<String>,
        #[arg(long, default_value = "mm")]
        unit: String,
        #[arg(long, default_value = "72")]
        dpi: f64,
        #[arg(long, short = 'p')]
        pixels: bool,
    },

    /// Look up Unicode characters by codepoint, name, range, or search term
    #[command(visible_aliases = ["g", "char"])]
    Glyph {
        /// A codepoint (U+0041 or 0x41) or a single character
        input: Option<String>,

        /// Named Unicode block/range (e.g. arrows, latin, greek)
        #[arg(long)]
        range: Option<String>,

        /// Search Unicode names matching this query
        #[arg(long)]
        search: Option<String>,

        /// Limit number of search results
        #[arg(long, default_value = "50")]
        limit: usize,
    },

    /// Extract metadata from a font file (ttf, otf, woff, woff2)
    #[command(visible_alias = "font")]
    FontInfo {
        font: PathBuf,
    },

    // ── Print & Production ──────────────────────────────────────────────────
    /// Analyse a PDF for print-readiness issues
    #[command(visible_alias = "pre")]
    Preflight {
        pdf: PathBuf,
    },

    /// Impose 8 images into a single-sheet mini-zine layout
    #[command(visible_alias = "z")]
    Zine {
        /// 8 page images (in reading order: page1..page8)
        images: Vec<PathBuf>,

        /// Output paper size (default: A4)
        #[arg(long, default_value = "a4")]
        paper: String,

        /// DPI for raster placement (default: 300)
        #[arg(long, default_value = "300")]
        dpi: f64,
    },

    /// Impose a PDF for booklet/saddle-stitch/n-up printing
    #[command(visible_aliases = ["imp", "i"])]
    Impose {
        pdf: PathBuf,

        /// Layout: saddle-stitch, perfect-bind, n-up
        #[arg(long, default_value = "saddle-stitch")]
        layout: String,

        /// Output paper size
        #[arg(long, default_value = "a4")]
        paper: String,

        /// Pages per sheet (for n-up; default 4)
        #[arg(long, default_value = "4")]
        n_up: u32,

        /// Pages per signature (for perfect-bind; default 16)
        #[arg(long, default_value = "16")]
        signature: u32,

        /// Margin in mm
        #[arg(long, default_value = "10")]
        margins: f64,

        /// Gutter in mm
        #[arg(long, default_value = "5")]
        gutter: f64,

        /// Creep compensation in mm
        #[arg(long, default_value = "0")]
        creep: f64,

        /// Draw crop marks
        #[arg(long)]
        crop_marks: bool,

        /// Add duplex back-sheet pages
        #[arg(long)]
        duplex: bool,
    },

    // ── Other Tools ─────────────────────────────────────────────────────────
    /// Generate styled QR codes
    #[command(visible_alias = "q")]
    Qr {
        /// Data to encode
        data: String,

        #[arg(long, default_value = "512")]
        size: u32,

        /// Foreground colour
        #[arg(long, default_value = "#000000")]
        fg: String,

        /// Background colour (use "transparent" for none)
        #[arg(long, default_value = "#ffffff")]
        bg: String,

        /// Optional logo to overlay (PNG, centered)
        #[arg(long)]
        logo: Option<PathBuf>,

        /// Error correction level: L, M, Q, H (default M)
        #[arg(long, default_value = "M")]
        error_level: String,
    },

    /// Generate 1D/2D barcodes
    #[command(visible_aliases = ["bc", "b"])]
    Barcode {
        data: String,

        /// Format: ean13, ean8, upca, code39, code128, codabar, code93, itf
        #[arg(long, default_value = "code128")]
        format: String,

        /// Bar height in px
        #[arg(long, default_value = "120")]
        height: u32,

        /// Width scale (px per module)
        #[arg(long, default_value = "2")]
        scale: u32,

        /// Include human-readable text below the barcode
        #[arg(long)]
        text: bool,
    },

    /// Generate HTML meta tags
    #[command(visible_alias = "og")]
    Meta {
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: String,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        image: Option<String>,
        #[arg(long, default_value = "website")]
        page_type: String,
        #[arg(long)]
        site_name: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        twitter_handle: Option<String>,
    },

    /// Test a regex pattern against text
    #[command(visible_aliases = ["re", "rx", "r"])]
    Regex {
        /// Regex pattern (Rust regex syntax)
        pattern: String,

        /// Text to match (or path / stdin)
        text: Option<String>,

        /// Flags: g (find all), i (case-insensitive), m (multi-line), s (dot-all), x (extended)
        #[arg(long, default_value = "g")]
        flags: String,
    },

    // ── Calculators ─────────────────────────────────────────────────────────
    /// Evaluate a mathematical expression
    #[command(visible_alias = "ca")]
    Calc {
        expression: Option<String>,

        /// Angle mode: deg or rad
        #[arg(long, default_value = "rad")]
        angles: String,
    },

    /// Convert numbers between bases (dec, hex, oct, bin)
    #[command(visible_alias = "radix")]
    Base {
        value: String,
        targets: Vec<String>,
        #[arg(long, default_value = "auto")]
        from: String,
    },

    /// Unix timestamp and date arithmetic
    #[command(visible_aliases = ["t", "date"])]
    Time {
        input: Option<String>,

        /// Output format(s): iso, rfc2822, rfc3339, unix, human. Default: all.
        #[arg(long)]
        to: Option<String>,

        /// Timezone (IANA name, e.g. America/New_York)
        #[arg(long)]
        tz: Option<String>,

        /// Add a duration (e.g. 30d, 5h, 90m)
        #[arg(long)]
        add: Option<String>,

        /// Subtract a duration
        #[arg(long)]
        sub: Option<String>,
    },

    /// Convert between units (length, weight, data, temperature, volume, speed, pressure, time)
    #[command(visible_alias = "u")]
    Unit {
        /// Value with unit, e.g. "100kg" or "100 kg"
        value: String,

        /// Target unit(s)
        targets: Vec<String>,
    },

    /// Encode text (base64, url)
    #[command(visible_aliases = ["enc", "e"])]
    Encode {
        encoding: String,
        input: Option<String>,
    },

    /// Decode text (base64, url)
    #[command(visible_aliases = ["dec", "d"])]
    Decode {
        encoding: String,
        input: Option<String>,
    },

    /// Generate a hash of input text
    #[command(visible_alias = "digest")]
    Hash {
        algorithm: String,
        input: Option<String>,
    },

    // ── Turbo-nerd shit ────────────────────────────────────────────────────
    /// Transliterate English text to the Shavian alphabet
    #[command(visible_aliases = ["shaw", "shav", "sv"])]
    Shavian {
        input: Option<String>,

        /// Show side-by-side gloss (Latin / Shavian / IPA)
        #[arg(long)]
        gloss: bool,
    },

    // ── meta ────────────────────────────────────────────────────────────────
    /// Generate shell completions
    Completions {
        shell: clap_complete::Shell,
    },

    /// Print a machine-readable usage reference for AI agents
    Agent,

    /// Install the bundled man pages so `man delphi` works
    InstallMan {
        /// Write to this directory instead of the default (`$HOME/.local/share/man/man1`)
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Print the paths that would be written without writing anything
        #[arg(long)]
        dry_run: bool,
    },
}
