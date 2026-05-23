pub mod colorblind;
pub mod contrast;
pub mod convert;
pub mod harmony;
pub mod names;
pub mod palette;
pub mod shades;

use crate::error::Error;

/// Colour stored as sRGB components (0.0–1.0) with alpha.
#[derive(Debug, Clone, Copy)]
pub struct Colour {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

// ---------------------------------------------------------------------------
// Construction & output
// ---------------------------------------------------------------------------

impl Colour {
    pub fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn with_alpha(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_u8(r: u8, g: u8, b: u8) -> Self {
        Self::new(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
    }

    pub fn to_u8(self) -> (u8, u8, u8) {
        (
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    pub fn to_hex(self) -> String {
        let (r, g, b) = self.to_u8();
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    }

    pub fn to_hex8(self) -> String {
        let (r, g, b) = self.to_u8();
        let a = (self.a.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
    }

    pub fn to_rgb_string(self) -> String {
        let (r, g, b) = self.to_u8();
        format!("rgb({}, {}, {})", r, g, b)
    }

    pub fn to_hsl_string(self) -> String {
        let (h, s, l) = rgb_to_hsl(self.r, self.g, self.b);
        format!("hsl({:.0}, {:.0}%, {:.0}%)", h, s * 100.0, l * 100.0)
    }

    pub fn to_oklch_string(self) -> String {
        let (l, a, b) = srgb_to_oklab(self.r, self.g, self.b);
        let (l, c, h) = oklab_to_oklch(l, a, b);
        format!("oklch({:.4} {:.4} {:.2})", l, c, h)
    }

    pub fn to_oklab_string(self) -> String {
        let (l, a, b) = srgb_to_oklab(self.r, self.g, self.b);
        format!("oklab({:.4} {:.4} {:.4})", l, a, b)
    }

    pub fn to_lab_string(self) -> String {
        let (x, y, z) = srgb_to_xyz(self.r, self.g, self.b);
        let (l, a, b) = xyz_to_lab(x, y, z);
        format!("lab({:.2} {:.2} {:.2})", l, a, b)
    }

    pub fn format_as(self, fmt: &str) -> Result<String, Error> {
        match fmt {
            "hex" => Ok(self.to_hex()),
            "hex8" => Ok(self.to_hex8()),
            "rgb" => Ok(self.to_rgb_string()),
            "hsl" => Ok(self.to_hsl_string()),
            "oklch" => Ok(self.to_oklch_string()),
            "oklab" => Ok(self.to_oklab_string()),
            "lab" => Ok(self.to_lab_string()),
            _ => Err(Error::Usage(format!("unknown colour format: {fmt}"))),
        }
    }

    /// WCAG relative luminance (using linearised sRGB).
    pub fn relative_luminance(self) -> f64 {
        let r = srgb_to_linear(self.r);
        let g = srgb_to_linear(self.g);
        let b = srgb_to_linear(self.b);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

impl Colour {
    pub fn parse(input: &str) -> Result<Self, Error> {
        let input = input.trim();

        None.or_else(|| Self::parse_hex(input))
            .or_else(|| Self::parse_rgb_func(input))
            .or_else(|| Self::parse_hsl_func(input))
            .or_else(|| Self::parse_oklch_func(input))
            .or_else(|| Self::parse_oklab_func(input))
            .or_else(|| {
                names::named_colour(input).map(|(r, g, b)| Self::from_u8(r, g, b))
            })
            .or_else(|| Self::parse_hex(&format!("#{input}")))
            .or_else(|| Self::parse_bare_rgb(input))
            .ok_or_else(|| Error::Input(format!("invalid colour: {input}")))
    }

    fn parse_hex(input: &str) -> Option<Self> {
        let hex = input.strip_prefix('#')?;
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                Some(Self::from_u8(r, g, b))
            }
            4 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?;
                Some(Self::with_alpha(
                    r as f64 / 255.0,
                    g as f64 / 255.0,
                    b as f64 / 255.0,
                    a as f64 / 255.0,
                ))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::from_u8(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::with_alpha(
                    r as f64 / 255.0,
                    g as f64 / 255.0,
                    b as f64 / 255.0,
                    a as f64 / 255.0,
                ))
            }
            _ => None,
        }
    }

    fn parse_rgb_func(input: &str) -> Option<Self> {
        let inner = input.strip_prefix("rgb(")?.strip_suffix(')')?;
        let parts: Vec<&str> = inner.split([',', ' ']).filter(|s| !s.is_empty()).collect();
        if parts.len() != 3 {
            return None;
        }
        let r: f64 = parts[0].trim().parse().ok()?;
        let g: f64 = parts[1].trim().parse().ok()?;
        let b: f64 = parts[2].trim().parse().ok()?;
        Some(Self::new(r / 255.0, g / 255.0, b / 255.0))
    }

    fn parse_hsl_func(input: &str) -> Option<Self> {
        let inner = input.strip_prefix("hsl(")?.strip_suffix(')')?;
        let parts: Vec<&str> = inner.split([',', ' ']).filter(|s| !s.is_empty()).collect();
        if parts.len() != 3 {
            return None;
        }
        let h: f64 = parts[0].trim().parse().ok()?;
        let s: f64 = parts[1].trim().trim_end_matches('%').parse::<f64>().ok()? / 100.0;
        let l: f64 = parts[2].trim().trim_end_matches('%').parse::<f64>().ok()? / 100.0;
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Some(Self::new(r, g, b))
    }

    fn parse_oklch_func(input: &str) -> Option<Self> {
        let inner = input.strip_prefix("oklch(")?.strip_suffix(')')?;
        let parts: Vec<&str> = inner.split([',', ' ']).filter(|s| !s.is_empty()).collect();
        if parts.len() != 3 {
            return None;
        }
        let l: f64 = parts[0].trim().parse().ok()?;
        let c: f64 = parts[1].trim().parse().ok()?;
        let h: f64 = parts[2].trim().parse().ok()?;
        let (lab_l, lab_a, lab_b) = oklch_to_oklab(l, c, h);
        let (r, g, b) = oklab_to_srgb(lab_l, lab_a, lab_b);
        Some(Self::new(
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            b.clamp(0.0, 1.0),
        ))
    }

    fn parse_oklab_func(input: &str) -> Option<Self> {
        let inner = input.strip_prefix("oklab(")?.strip_suffix(')')?;
        let parts: Vec<&str> = inner.split([',', ' ']).filter(|s| !s.is_empty()).collect();
        if parts.len() != 3 {
            return None;
        }
        let l: f64 = parts[0].trim().parse().ok()?;
        let a: f64 = parts[1].trim().parse().ok()?;
        let b: f64 = parts[2].trim().parse().ok()?;
        let (r, g, bb) = oklab_to_srgb(l, a, b);
        Some(Self::new(
            r.clamp(0.0, 1.0),
            g.clamp(0.0, 1.0),
            bb.clamp(0.0, 1.0),
        ))
    }

    fn parse_bare_rgb(input: &str) -> Option<Self> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() != 3 {
            return None;
        }
        let r: f64 = parts[0].parse().ok()?;
        let g: f64 = parts[1].parse().ok()?;
        let b: f64 = parts[2].parse().ok()?;
        if r > 255.0 || g > 255.0 || b > 255.0 || r < 0.0 || g < 0.0 || b < 0.0 {
            return None;
        }
        Some(Self::new(r / 255.0, g / 255.0, b / 255.0))
    }
}

// ---------------------------------------------------------------------------
// Colour-space maths
// ---------------------------------------------------------------------------

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn srgb_to_xyz(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let r = srgb_to_linear(r);
    let g = srgb_to_linear(g);
    let b = srgb_to_linear(b);
    (
        0.4124564 * r + 0.3575761 * g + 0.1804375 * b,
        0.2126729 * r + 0.7151522 * g + 0.0721750 * b,
        0.0193339 * r + 0.1191920 * g + 0.9503041 * b,
    )
}

#[allow(dead_code)]
fn xyz_to_srgb(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let r = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
    let g = -0.9692660 * x + 1.8760108 * y + 0.0415560 * z;
    let b = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;
    (linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b))
}

// OKLab ↔ linear sRGB (direct path, per Björn Ottosson's spec)

pub(crate) fn srgb_to_oklab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let r = srgb_to_linear(r);
    let g = srgb_to_linear(g);
    let b = srgb_to_linear(b);

    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    (
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    )
}

pub(crate) fn oklab_to_srgb(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    (linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b))
}

pub(crate) fn oklab_to_oklch(l: f64, a: f64, b: f64) -> (f64, f64, f64) {
    let c = (a * a + b * b).sqrt();
    let h = b.atan2(a).to_degrees();
    (l, c, if h < 0.0 { h + 360.0 } else { h })
}

pub(crate) fn oklch_to_oklab(l: f64, c: f64, h: f64) -> (f64, f64, f64) {
    let h_rad = h.to_radians();
    (l, c * h_rad.cos(), c * h_rad.sin())
}

// CIE Lab ↔ XYZ (D65 illuminant)

const XN: f64 = 0.95047;
const YN: f64 = 1.0;
const ZN: f64 = 1.08883;

fn lab_f(t: f64) -> f64 {
    const DELTA: f64 = 6.0 / 29.0;
    if t > DELTA * DELTA * DELTA {
        t.cbrt()
    } else {
        t / (3.0 * DELTA * DELTA) + 4.0 / 29.0
    }
}

fn xyz_to_lab(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let fx = lab_f(x / XN);
    let fy = lab_f(y / YN);
    let fz = lab_f(z / ZN);
    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

// HSL ↔ sRGB

pub(crate) fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - r).abs() < f64::EPSILON {
        ((g - b) / d) + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };

    (h * 60.0, s, l)
}

pub(crate) fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    if s.abs() < f64::EPSILON {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h = h / 360.0;
    (
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
    )
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    // --- Parsing ---

    #[test]
    fn parse_hex6() {
        let c = Colour::parse("#ff6600").unwrap();
        assert_eq!(c.to_u8(), (255, 102, 0));
    }

    #[test]
    fn parse_hex3() {
        let c = Colour::parse("#f60").unwrap();
        assert_eq!(c.to_u8(), (255, 102, 0));
    }

    #[test]
    fn parse_hex8() {
        let c = Colour::parse("#ff660080").unwrap();
        assert_eq!(c.to_u8(), (255, 102, 0));
        assert!(approx(c.a, 128.0 / 255.0, 0.01));
    }

    #[test]
    fn parse_hex4() {
        let c = Colour::parse("#f608").unwrap();
        assert_eq!(c.to_u8(), (255, 102, 0));
        assert!(approx(c.a, 0x88 as f64 / 255.0, 0.01));
    }

    #[test]
    fn parse_bare_hex() {
        let c = Colour::parse("ff6600").unwrap();
        assert_eq!(c.to_u8(), (255, 102, 0));
    }

    #[test]
    fn parse_rgb_func() {
        let c = Colour::parse("rgb(255, 102, 0)").unwrap();
        assert_eq!(c.to_u8(), (255, 102, 0));
    }

    #[test]
    fn parse_rgb_func_spaces() {
        let c = Colour::parse("rgb(255 102 0)").unwrap();
        assert_eq!(c.to_u8(), (255, 102, 0));
    }

    #[test]
    fn parse_hsl_func() {
        let c = Colour::parse("hsl(0, 100%, 50%)").unwrap();
        assert_eq!(c.to_u8(), (255, 0, 0));
    }

    #[test]
    fn parse_hsl_green() {
        let c = Colour::parse("hsl(120, 100%, 50%)").unwrap();
        assert_eq!(c.to_u8(), (0, 255, 0));
    }

    #[test]
    fn parse_oklch_roundtrip() {
        // Convert #ff6600 → oklch string → parse back, should be close
        let orig = Colour::from_u8(255, 102, 0);
        let oklch_str = orig.to_oklch_string();
        let parsed = Colour::parse(&oklch_str).unwrap();
        let (r, g, b) = parsed.to_u8();
        assert!((r as i32 - 255).abs() <= 1);
        assert!((g as i32 - 102).abs() <= 1);
        assert!((b as i32 - 0).abs() <= 1);
    }

    #[test]
    fn parse_oklab_roundtrip() {
        let orig = Colour::from_u8(255, 102, 0);
        let oklab_str = orig.to_oklab_string();
        let parsed = Colour::parse(&oklab_str).unwrap();
        let (r, g, b) = parsed.to_u8();
        assert!((r as i32 - 255).abs() <= 1);
        assert!((g as i32 - 102).abs() <= 1);
        assert!((b as i32 - 0).abs() <= 1);
    }

    #[test]
    fn parse_named_colour() {
        let c = Colour::parse("rebeccapurple").unwrap();
        assert_eq!(c.to_u8(), (102, 51, 153));
    }

    #[test]
    fn parse_named_case_insensitive() {
        let c = Colour::parse("RebeccaPurple").unwrap();
        assert_eq!(c.to_u8(), (102, 51, 153));
    }

    #[test]
    fn parse_bare_rgb_numbers() {
        let c = Colour::parse("255 102 0").unwrap();
        assert_eq!(c.to_u8(), (255, 102, 0));
    }

    #[test]
    fn parse_invalid() {
        assert!(Colour::parse("not-a-colour").is_err());
        assert!(Colour::parse("").is_err());
        assert!(Colour::parse("#gggggg").is_err());
    }

    // --- Output formats ---

    #[test]
    fn to_hex_roundtrip() {
        let c = Colour::from_u8(255, 102, 0);
        assert_eq!(c.to_hex(), "#ff6600");
    }

    #[test]
    fn to_hex8_full_alpha() {
        let c = Colour::from_u8(255, 102, 0);
        assert_eq!(c.to_hex8(), "#ff6600ff");
    }

    #[test]
    fn to_rgb_string() {
        let c = Colour::from_u8(255, 102, 0);
        assert_eq!(c.to_rgb_string(), "rgb(255, 102, 0)");
    }

    #[test]
    fn to_hsl_string_red() {
        let c = Colour::from_u8(255, 0, 0);
        assert_eq!(c.to_hsl_string(), "hsl(0, 100%, 50%)");
    }

    #[test]
    fn format_as_unknown() {
        let c = Colour::from_u8(0, 0, 0);
        assert!(c.format_as("nope").is_err());
    }

    // --- Luminance ---

    #[test]
    fn luminance_black() {
        let c = Colour::from_u8(0, 0, 0);
        assert!(approx(c.relative_luminance(), 0.0, 0.001));
    }

    #[test]
    fn luminance_white() {
        let c = Colour::from_u8(255, 255, 255);
        assert!(approx(c.relative_luminance(), 1.0, 0.001));
    }

    // --- Colour space maths ---

    #[test]
    fn hsl_roundtrip() {
        let (h, s, l) = rgb_to_hsl(1.0, 0.0, 0.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!(approx(r, 1.0, 0.001));
        assert!(approx(g, 0.0, 0.001));
        assert!(approx(b, 0.0, 0.001));
    }

    #[test]
    fn hsl_grey() {
        let (h, s, l) = rgb_to_hsl(0.5, 0.5, 0.5);
        assert!(approx(s, 0.0, 0.001));
        assert!(approx(l, 0.5, 0.001));
        let _ = h; // hue is undefined for grey
    }

    #[test]
    fn oklab_roundtrip() {
        let (l, a, b) = srgb_to_oklab(1.0, 0.4, 0.0);
        let (r, g, bb) = oklab_to_srgb(l, a, b);
        assert!(approx(r, 1.0, 0.001));
        assert!(approx(g, 0.4, 0.001));
        assert!(approx(bb, 0.0, 0.001));
    }

    #[test]
    fn oklch_roundtrip() {
        let (l, a, b) = (0.7, 0.15, 45.0_f64.to_radians().sin() * 0.15);
        let (ll, c, h) = oklab_to_oklch(l, a, b);
        let (l2, a2, b2) = oklch_to_oklab(ll, c, h);
        assert!(approx(l, l2, 0.001));
        assert!(approx(a, a2, 0.001));
        assert!(approx(b, b2, 0.001));
    }

    #[test]
    fn srgb_xyz_roundtrip() {
        let (x, y, z) = srgb_to_xyz(0.8, 0.3, 0.1);
        let (r, g, b) = xyz_to_srgb(x, y, z);
        assert!(approx(r, 0.8, 0.001));
        assert!(approx(g, 0.3, 0.001));
        assert!(approx(b, 0.1, 0.001));
    }
}
