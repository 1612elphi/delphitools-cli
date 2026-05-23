use crate::colour::{oklab_to_srgb, oklch_to_oklab, Colour};
use crate::error::Error;
use crate::output;
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde_json::json;
use std::path::Path;

// ============================================================================
// CATEGORY & STRATEGY
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Random,
    ColorTheory,
    Mood,
    Era,
    Nature,
    Cultural,
}

impl Category {
    fn title(self) -> &'static str {
        match self {
            Category::Random => "Random",
            Category::ColorTheory => "Color Theory",
            Category::Mood => "Moods",
            Category::Era => "Decades & Eras",
            Category::Nature => "Nature & Scenes",
            Category::Cultural => "Art & Culture",
        }
    }

    fn order() -> &'static [Category] {
        &[
            Category::Random,
            Category::ColorTheory,
            Category::Mood,
            Category::Era,
            Category::Nature,
            Category::Cultural,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    // Random
    TrueRandom,
    RandomCohesive,
    // Color theory
    Analogous,
    Complementary,
    Triadic,
    SplitComplementary,
    Tetradic,
    Monochromatic,
    // Mood
    Thermos,
    Specimen,
    Souvenir,
    Curfew,
    Telegraph,
    // Era
    Seventies,
    Eighties,
    Nineties,
    Y2K,
    // Nature
    OceanSunset,
    ForestMorning,
    DesertDusk,
    Arctic,
    Volcanic,
    Meadow,
    // Cultural
    Bauhaus,
    ArtDeco,
    Japanese,
    Scandinavian,
    Mexican,
}

impl Strategy {
    fn all() -> &'static [Strategy] {
        &[
            Strategy::TrueRandom,
            Strategy::RandomCohesive,
            Strategy::Analogous,
            Strategy::Complementary,
            Strategy::Triadic,
            Strategy::SplitComplementary,
            Strategy::Tetradic,
            Strategy::Monochromatic,
            Strategy::Thermos,
            Strategy::Specimen,
            Strategy::Souvenir,
            Strategy::Curfew,
            Strategy::Telegraph,
            Strategy::Seventies,
            Strategy::Eighties,
            Strategy::Nineties,
            Strategy::Y2K,
            Strategy::OceanSunset,
            Strategy::ForestMorning,
            Strategy::DesertDusk,
            Strategy::Arctic,
            Strategy::Volcanic,
            Strategy::Meadow,
            Strategy::Bauhaus,
            Strategy::ArtDeco,
            Strategy::Japanese,
            Strategy::Scandinavian,
            Strategy::Mexican,
        ]
    }

    /// (slug, display name, description, category)
    fn info(self) -> (&'static str, &'static str, &'static str, Category) {
        match self {
            // Random
            Strategy::TrueRandom => (
                "true-random",
                "Chaos",
                "Completely random, no rules",
                Category::Random,
            ),
            Strategy::RandomCohesive => (
                "random-cohesive",
                "Random",
                "Random cohesive palette",
                Category::Random,
            ),
            // Color theory
            Strategy::Analogous => (
                "analogous",
                "Analogous",
                "Adjacent hues on the colour wheel",
                Category::ColorTheory,
            ),
            Strategy::Complementary => (
                "complementary",
                "Complementary",
                "Opposite hues for high contrast",
                Category::ColorTheory,
            ),
            Strategy::Triadic => (
                "triadic",
                "Triadic",
                "Three evenly spaced hues",
                Category::ColorTheory,
            ),
            Strategy::SplitComplementary => (
                "split-complementary",
                "Split-Comp",
                "Base + two adjacent to complement",
                Category::ColorTheory,
            ),
            Strategy::Tetradic => (
                "tetradic",
                "Tetradic",
                "Four evenly spaced hues",
                Category::ColorTheory,
            ),
            Strategy::Monochromatic => (
                "monochromatic",
                "Mono",
                "Single hue, varied lightness",
                Category::ColorTheory,
            ),
            // Mood
            Strategy::Thermos => (
                "thermos",
                "Thermos",
                "Warm, cozy, retro tones",
                Category::Mood,
            ),
            Strategy::Specimen => (
                "specimen",
                "Specimen",
                "Cool, clinical, preserved",
                Category::Mood,
            ),
            Strategy::Souvenir => (
                "souvenir",
                "Souvenir",
                "Soft, faded pastels",
                Category::Mood,
            ),
            Strategy::Curfew => ("curfew", "Curfew", "Dark, moody depths", Category::Mood),
            Strategy::Telegraph => (
                "telegraph",
                "Telegraph",
                "Muted vintage sepia",
                Category::Mood,
            ),
            // Era
            Strategy::Seventies => (
                "70s",
                "1970s",
                "Earth tones, burnt orange, avocado",
                Category::Era,
            ),
            Strategy::Eighties => (
                "80s",
                "1980s",
                "Neon pink, electric blue, hot purple",
                Category::Era,
            ),
            Strategy::Nineties => (
                "90s",
                "1990s",
                "Grunge, forest green, burgundy",
                Category::Era,
            ),
            Strategy::Y2K => ("y2k", "Y2K", "Chrome, cyan, magenta", Category::Era),
            // Nature
            Strategy::OceanSunset => (
                "ocean-sunset",
                "Ocean Sunset",
                "Coral, rose, ocean blue, dusk",
                Category::Nature,
            ),
            Strategy::ForestMorning => (
                "forest-morning",
                "Forest Morning",
                "Fresh greens, mist, golden light",
                Category::Nature,
            ),
            Strategy::DesertDusk => (
                "desert-dusk",
                "Desert Dusk",
                "Terracotta, sand, dusty rose",
                Category::Nature,
            ),
            Strategy::Arctic => (
                "arctic",
                "Arctic",
                "Ice blue, white, pale cyan",
                Category::Nature,
            ),
            Strategy::Volcanic => (
                "volcanic",
                "Volcanic",
                "Black, deep red, orange, ash",
                Category::Nature,
            ),
            Strategy::Meadow => (
                "meadow",
                "Meadow",
                "Grass green, wildflowers, sky blue",
                Category::Nature,
            ),
            // Cultural
            Strategy::Bauhaus => (
                "bauhaus",
                "Bauhaus",
                "Primary colors, geometric, bold",
                Category::Cultural,
            ),
            Strategy::ArtDeco => (
                "art-deco",
                "Art Deco",
                "Gold, black, cream, emerald",
                Category::Cultural,
            ),
            Strategy::Japanese => (
                "japanese",
                "Japanese",
                "Indigo, vermillion, gold, cream",
                Category::Cultural,
            ),
            Strategy::Scandinavian => (
                "scandinavian",
                "Scandinavian",
                "White, pale grey, muted pastels",
                Category::Cultural,
            ),
            Strategy::Mexican => (
                "mexican",
                "Mexican",
                "Hot pink, orange, turquoise, yellow",
                Category::Cultural,
            ),
        }
    }

    pub fn parse(s: &str) -> Result<Self, Error> {
        let key = s.to_lowercase();
        for &strat in Strategy::all() {
            if strat.info().0 == key {
                return Ok(strat);
            }
        }
        Err(Error::Usage(format!(
            "unknown strategy: {s}\nuse --list to see all available strategies"
        )))
    }

    fn category(self) -> Category {
        self.info().3
    }

    pub fn generate(self, count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
        match self {
            // Random
            Strategy::TrueRandom => true_random(count, rng),
            Strategy::RandomCohesive => random_cohesive(count, rng),
            // Color theory
            Strategy::Analogous => analogous(count, rng),
            Strategy::Complementary => complementary(count, rng),
            Strategy::Triadic => triadic(count, rng),
            Strategy::SplitComplementary => split_complementary(count, rng),
            Strategy::Tetradic => tetradic(count, rng),
            Strategy::Monochromatic => monochromatic(count, rng),
            // Mood
            Strategy::Thermos => thermos(count, rng),
            Strategy::Specimen => specimen(count, rng),
            Strategy::Souvenir => souvenir(count, rng),
            Strategy::Curfew => curfew(count, rng),
            Strategy::Telegraph => telegraph(count, rng),
            // Era
            Strategy::Seventies => seventies(count, rng),
            Strategy::Eighties => eighties(count, rng),
            Strategy::Nineties => nineties(count, rng),
            Strategy::Y2K => y2k(count, rng),
            // Nature
            Strategy::OceanSunset => ocean_sunset(count, rng),
            Strategy::ForestMorning => forest_morning(count, rng),
            Strategy::DesertDusk => desert_dusk(count, rng),
            Strategy::Arctic => arctic(count, rng),
            Strategy::Volcanic => volcanic(count, rng),
            Strategy::Meadow => meadow(count, rng),
            // Cultural
            Strategy::Bauhaus => bauhaus(count, rng),
            Strategy::ArtDeco => art_deco(count, rng),
            Strategy::Japanese => japanese(count, rng),
            Strategy::Scandinavian => scandinavian(count, rng),
            Strategy::Mexican => mexican(count, rng),
        }
    }
}

// ============================================================================
// RNG WRAPPER
// ============================================================================

enum AnyRng {
    Seeded(ChaCha8Rng),
    Thread(ThreadRng),
}

impl RngCore for AnyRng {
    fn next_u32(&mut self) -> u32 {
        match self {
            AnyRng::Seeded(r) => r.next_u32(),
            AnyRng::Thread(r) => r.next_u32(),
        }
    }
    fn next_u64(&mut self) -> u64 {
        match self {
            AnyRng::Seeded(r) => r.next_u64(),
            AnyRng::Thread(r) => r.next_u64(),
        }
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        match self {
            AnyRng::Seeded(r) => r.fill_bytes(dest),
            AnyRng::Thread(r) => r.fill_bytes(dest),
        }
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        match self {
            AnyRng::Seeded(r) => r.try_fill_bytes(dest),
            AnyRng::Thread(r) => r.try_fill_bytes(dest),
        }
    }
}

fn build_rng(seed: Option<u64>) -> AnyRng {
    match seed {
        Some(s) => AnyRng::Seeded(ChaCha8Rng::seed_from_u64(s)),
        None => AnyRng::Thread(thread_rng()),
    }
}

// ============================================================================
// COLOUR UTILITIES
// ============================================================================

/// Mirrors TS `randomInRange`. Note: when min > max (e.g. wrap-around hue ranges
/// like `[350, 20]`), this returns values in `[max, min]` — matching TS behaviour
/// where `Math.random() * (max - min) + min` becomes negative and silently shifts.
fn random_in_range(rng: &mut dyn RngCore, min: f64, max: f64) -> f64 {
    let t: f64 = rng.gen();
    t * (max - min) + min
}

fn clamp_oklch(l: f64, c: f64, h: f64) -> (f64, f64, f64) {
    let l = l.clamp(0.0, 1.0);
    let c = c.clamp(0.0, 0.4);
    let h = ((h % 360.0) + 360.0) % 360.0;
    (l, c, h)
}

fn oklch_to_colour(l: f64, c: f64, h: f64) -> Colour {
    let (cl, cc, ch) = clamp_oklch(l, c, h);
    let (lab_l, lab_a, lab_b) = oklch_to_oklab(cl, cc, ch);
    let (r, g, b) = oklab_to_srgb(lab_l, lab_a, lab_b);
    Colour::new(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

fn random_base(rng: &mut dyn RngCore) -> (f64, f64, f64) {
    let l = random_in_range(rng, 0.4, 0.75);
    let c = random_in_range(rng, 0.08, 0.2);
    let h = random_in_range(rng, 0.0, 360.0);
    (l, c, h)
}

#[derive(Clone, Copy)]
struct HueRange {
    h: (f64, f64),
    weight: f64,
    l: Option<(f64, f64)>,
    c: Option<(f64, f64)>,
}

impl HueRange {
    const fn new(h: (f64, f64), weight: f64) -> Self {
        Self { h, weight, l: None, c: None }
    }
    const fn with_l(h: (f64, f64), weight: f64, l: (f64, f64)) -> Self {
        Self { h, weight, l: Some(l), c: None }
    }
    const fn with_lc(h: (f64, f64), weight: f64, l: (f64, f64), c: (f64, f64)) -> Self {
        Self { h, weight, l: Some(l), c: Some(c) }
    }
}

fn pick_from_hue_ranges(
    rng: &mut dyn RngCore,
    ranges: &[HueRange],
    default_l: (f64, f64),
    default_c: (f64, f64),
) -> Colour {
    let total_weight: f64 = ranges.iter().map(|r| r.weight).sum();
    let mut roll = rng.gen::<f64>() * total_weight;
    let mut selected = ranges[0];
    for &range in ranges {
        roll -= range.weight;
        if roll <= 0.0 {
            selected = range;
            break;
        }
    }
    let h = random_in_range(rng, selected.h.0, selected.h.1);
    let lrange = selected.l.unwrap_or(default_l);
    let crange = selected.c.unwrap_or(default_c);
    let l = random_in_range(rng, lrange.0, lrange.1);
    let c = random_in_range(rng, crange.0, crange.1);
    oklch_to_colour(l, c, h)
}

// ============================================================================
// RANDOM
// ============================================================================

fn true_random(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    (0..count)
        .map(|_| {
            let r: u8 = rng.gen();
            let g: u8 = rng.gen();
            let b: u8 = rng.gen();
            Colour::from_u8(r, g, b)
        })
        .collect()
}

fn random_cohesive(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let strategies: [fn(usize, &mut dyn RngCore) -> Vec<Colour>; 6] = [
        analogous,
        complementary,
        triadic,
        split_complementary,
        tetradic,
        monochromatic,
    ];
    let idx = (rng.gen::<f64>() * strategies.len() as f64).floor() as usize;
    let idx = idx.min(strategies.len() - 1);
    strategies[idx](count, rng)
}

// ============================================================================
// COLOR THEORY
// ============================================================================

fn analogous(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let (base_l, base_c, base_h) = random_base(rng);
    let spread = 40.0;
    let denom = (count as f64 - 1.0).max(1.0);
    let step = spread / denom;
    let start_h = base_h - spread / 2.0;
    (0..count)
        .map(|i| {
            let h = start_h + step * i as f64;
            let l = base_l + random_in_range(rng, -0.1, 0.1);
            let c = base_c + random_in_range(rng, -0.05, 0.05);
            oklch_to_colour(l, c, h)
        })
        .collect()
}

fn complementary(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let (base_l, base_c, base_h) = random_base(rng);
    let complement_h = (base_h + 180.0) % 360.0;
    let half = (count + 1) / 2; // Math.ceil(count / 2)
    let mut out = Vec::with_capacity(count);
    for _ in 0..half {
        let h_var = random_in_range(rng, -15.0, 15.0);
        let l = base_l + random_in_range(rng, -0.15, 0.15);
        let c = base_c + random_in_range(rng, -0.05, 0.05);
        out.push(oklch_to_colour(l, c, base_h + h_var));
    }
    for _ in half..count {
        let h_var = random_in_range(rng, -15.0, 15.0);
        let l = base_l + random_in_range(rng, -0.15, 0.15);
        let c = base_c + random_in_range(rng, -0.05, 0.05);
        out.push(oklch_to_colour(l, c, complement_h + h_var));
    }
    out
}

fn triadic(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let (base_l, base_c, base_h) = random_base(rng);
    let angles = [base_h, (base_h + 120.0) % 360.0, (base_h + 240.0) % 360.0];
    (0..count)
        .map(|i| {
            let h = angles[i % 3] + random_in_range(rng, -10.0, 10.0);
            let l = base_l + random_in_range(rng, -0.15, 0.15);
            let c = base_c + random_in_range(rng, -0.05, 0.05);
            oklch_to_colour(l, c, h)
        })
        .collect()
}

fn split_complementary(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let (base_l, base_c, base_h) = random_base(rng);
    let split1 = (base_h + 150.0) % 360.0;
    let split2 = (base_h + 210.0) % 360.0;
    let angles = [base_h, split1, split2];
    (0..count)
        .map(|i| {
            let h = angles[i % 3] + random_in_range(rng, -10.0, 10.0);
            let l = base_l + random_in_range(rng, -0.15, 0.15);
            let c = base_c + random_in_range(rng, -0.05, 0.05);
            oklch_to_colour(l, c, h)
        })
        .collect()
}

fn tetradic(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let (base_l, base_c, base_h) = random_base(rng);
    let angles = [
        base_h,
        (base_h + 90.0) % 360.0,
        (base_h + 180.0) % 360.0,
        (base_h + 270.0) % 360.0,
    ];
    (0..count)
        .map(|i| {
            let h = angles[i % 4] + random_in_range(rng, -10.0, 10.0);
            let l = base_l + random_in_range(rng, -0.15, 0.15);
            let c = base_c + random_in_range(rng, -0.05, 0.05);
            oklch_to_colour(l, c, h)
        })
        .collect()
}

fn monochromatic(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let h = random_in_range(rng, 0.0, 360.0);
    let base_c = random_in_range(rng, 0.1, 0.2);
    let l_min = 0.3;
    let l_max = 0.85;
    let denom = (count as f64 - 1.0).max(1.0);
    let l_step = (l_max - l_min) / denom;
    (0..count)
        .map(|i| {
            let l = l_max - l_step * i as f64;
            let c_mod = if l < 0.4 || l > 0.75 { 0.7 } else { 1.0 };
            oklch_to_colour(l, base_c * c_mod, h)
        })
        .collect()
}

// ============================================================================
// MOOD
// ============================================================================

fn simple_oklch_palette(
    count: usize,
    rng: &mut dyn RngCore,
    h_range: (f64, f64),
    l_range: (f64, f64),
    c_range: (f64, f64),
) -> Vec<Colour> {
    (0..count)
        .map(|_| {
            let h = random_in_range(rng, h_range.0, h_range.1);
            let l = random_in_range(rng, l_range.0, l_range.1);
            let c = random_in_range(rng, c_range.0, c_range.1);
            oklch_to_colour(l, c, h)
        })
        .collect()
}

fn thermos(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    simple_oklch_palette(count, rng, (15.0, 55.0), (0.45, 0.75), (0.08, 0.18))
}

fn specimen(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    simple_oklch_palette(count, rng, (170.0, 220.0), (0.6, 0.9), (0.03, 0.12))
}

fn souvenir(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    simple_oklch_palette(count, rng, (0.0, 360.0), (0.75, 0.92), (0.04, 0.10))
}

fn curfew(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    simple_oklch_palette(count, rng, (0.0, 360.0), (0.15, 0.35), (0.05, 0.15))
}

fn telegraph(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    simple_oklch_palette(count, rng, (30.0, 60.0), (0.4, 0.7), (0.02, 0.08))
}

// ============================================================================
// ERA
// ============================================================================

fn seventies(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let ranges = [
        HueRange::new((25.0, 45.0), 3.0),
        HueRange::new((75.0, 100.0), 2.0),
        HueRange::new((15.0, 30.0), 2.0),
        HueRange::new((45.0, 65.0), 1.0),
    ];
    (0..count)
        .map(|_| pick_from_hue_ranges(rng, &ranges, (0.35, 0.65), (0.08, 0.18)))
        .collect()
}

fn eighties(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let bright_ranges = [
        HueRange::new((320.0, 350.0), 3.0),
        HueRange::new((220.0, 270.0), 2.0),
        HueRange::new((280.0, 320.0), 2.0),
        HueRange::new((170.0, 200.0), 1.0),
    ];
    (0..count)
        .map(|_| {
            if rng.gen::<f64>() < 0.2 {
                let h = random_in_range(rng, 0.0, 360.0);
                oklch_to_colour(
                    random_in_range(rng, 0.12, 0.22),
                    random_in_range(rng, 0.02, 0.08),
                    h,
                )
            } else {
                pick_from_hue_ranges(rng, &bright_ranges, (0.55, 0.75), (0.18, 0.30))
            }
        })
        .collect()
}

fn nineties(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    // Note: `(350, 20)` is a wrap range — preserved literally per TS reference.
    let ranges = [
        HueRange::new((140.0, 170.0), 2.0),
        HueRange::new((350.0, 20.0), 2.0),
        HueRange::new((220.0, 250.0), 2.0),
        HueRange::new((30.0, 50.0), 1.0),
    ];
    (0..count)
        .map(|_| pick_from_hue_ranges(rng, &ranges, (0.30, 0.55), (0.05, 0.14)))
        .collect()
}

fn y2k(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let bright_ranges = [
        HueRange::new((180.0, 200.0), 2.0),
        HueRange::new((310.0, 340.0), 2.0),
        HueRange::new((260.0, 290.0), 1.0),
        HueRange::new((50.0, 70.0), 1.0),
    ];
    (0..count)
        .map(|_| {
            if rng.gen::<f64>() < 0.3 {
                let h = random_in_range(rng, 200.0, 280.0);
                oklch_to_colour(
                    random_in_range(rng, 0.7, 0.88),
                    random_in_range(rng, 0.01, 0.04),
                    h,
                )
            } else {
                pick_from_hue_ranges(rng, &bright_ranges, (0.55, 0.75), (0.15, 0.28))
            }
        })
        .collect()
}

// ============================================================================
// NATURE
// ============================================================================

fn ocean_sunset(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let ranges = [
        HueRange::with_l((15.0, 40.0), 2.0, (0.6, 0.75)),
        HueRange::with_l((340.0, 360.0), 2.0, (0.55, 0.7)),
        HueRange::with_l((200.0, 230.0), 2.0, (0.35, 0.55)),
        HueRange::with_l((260.0, 290.0), 1.0, (0.25, 0.45)),
    ];
    (0..count)
        .map(|_| pick_from_hue_ranges(rng, &ranges, (0.45, 0.7), (0.1, 0.2)))
        .collect()
}

fn forest_morning(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let ranges = [
        HueRange::new((100.0, 140.0), 3.0),
        HueRange::new((75.0, 100.0), 2.0),
        HueRange::new((45.0, 60.0), 1.0),
        HueRange::new((25.0, 40.0), 1.0),
    ];
    (0..count)
        .map(|_| {
            if rng.gen::<f64>() < 0.25 {
                let h = random_in_range(rng, 90.0, 150.0);
                oklch_to_colour(
                    random_in_range(rng, 0.8, 0.92),
                    random_in_range(rng, 0.02, 0.06),
                    h,
                )
            } else {
                pick_from_hue_ranges(rng, &ranges, (0.4, 0.7), (0.08, 0.18))
            }
        })
        .collect()
}

fn desert_dusk(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    // `(350, 15)` is a wrap range — preserved literally per TS reference.
    let ranges = [
        HueRange::with_l((15.0, 35.0), 3.0, (0.45, 0.65)),
        HueRange::with_l((40.0, 55.0), 2.0, (0.7, 0.85)),
        HueRange::with_l((350.0, 15.0), 2.0, (0.55, 0.7)),
        HueRange::with_l((280.0, 310.0), 1.0, (0.25, 0.4)),
    ];
    (0..count)
        .map(|_| pick_from_hue_ranges(rng, &ranges, (0.45, 0.7), (0.06, 0.16)))
        .collect()
}

fn arctic(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let ranges = [
        HueRange::new((200.0, 220.0), 3.0),
        HueRange::new((180.0, 200.0), 2.0),
        HueRange::new((220.0, 250.0), 1.0),
    ];
    (0..count)
        .map(|_| {
            if rng.gen::<f64>() < 0.3 {
                let h = random_in_range(rng, 200.0, 220.0);
                oklch_to_colour(
                    random_in_range(rng, 0.92, 0.98),
                    random_in_range(rng, 0.005, 0.02),
                    h,
                )
            } else {
                pick_from_hue_ranges(rng, &ranges, (0.7, 0.9), (0.02, 0.08))
            }
        })
        .collect()
}

fn volcanic(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let hot_ranges = [
        HueRange::new((0.0, 20.0), 2.0),
        HueRange::new((20.0, 45.0), 2.0),
        HueRange::new((45.0, 60.0), 1.0),
    ];
    (0..count)
        .map(|_| {
            let roll: f64 = rng.gen();
            if roll < 0.25 {
                let h = random_in_range(rng, 0.0, 360.0);
                oklch_to_colour(
                    random_in_range(rng, 0.12, 0.22),
                    random_in_range(rng, 0.01, 0.03),
                    h,
                )
            } else if roll < 0.4 {
                let h = random_in_range(rng, 20.0, 40.0);
                oklch_to_colour(
                    random_in_range(rng, 0.5, 0.65),
                    random_in_range(rng, 0.01, 0.03),
                    h,
                )
            } else {
                pick_from_hue_ranges(rng, &hot_ranges, (0.4, 0.65), (0.15, 0.25))
            }
        })
        .collect()
}

fn meadow(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let ranges = [
        HueRange::new((100.0, 135.0), 3.0),
        HueRange::new((280.0, 320.0), 2.0),
        HueRange::new((55.0, 75.0), 2.0),
        HueRange::new((200.0, 220.0), 1.0),
    ];
    (0..count)
        .map(|_| pick_from_hue_ranges(rng, &ranges, (0.55, 0.75), (0.12, 0.22)))
        .collect()
}

// ============================================================================
// CULTURAL
// ============================================================================

fn bauhaus(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let ranges = [
        HueRange::with_lc((15.0, 35.0), 3.0, (0.5, 0.62), (0.18, 0.26)),
        HueRange::with_lc((85.0, 105.0), 3.0, (0.8, 0.88), (0.14, 0.2)),
        HueRange::with_lc((240.0, 265.0), 3.0, (0.4, 0.52), (0.12, 0.18)),
        HueRange::with_lc((35.0, 55.0), 1.0, (0.65, 0.75), (0.15, 0.2)),
        HueRange::with_lc((140.0, 160.0), 1.0, (0.45, 0.55), (0.1, 0.15)),
        HueRange::with_lc((0.0, 15.0), 1.0, (0.45, 0.55), (0.2, 0.26)),
    ];
    (0..count)
        .map(|_| {
            let roll: f64 = rng.gen();
            if roll < 0.2 {
                let h = random_in_range(rng, 0.0, 360.0);
                oklch_to_colour(
                    random_in_range(rng, 0.08, 0.18),
                    random_in_range(rng, 0.0, 0.02),
                    h,
                )
            } else if roll < 0.3 {
                let h = random_in_range(rng, 80.0, 100.0);
                oklch_to_colour(
                    random_in_range(rng, 0.92, 0.97),
                    random_in_range(rng, 0.01, 0.025),
                    h,
                )
            } else {
                pick_from_hue_ranges(rng, &ranges, (0.5, 0.7), (0.15, 0.22))
            }
        })
        .collect()
}

fn art_deco(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let jewel = [
        HueRange::new((155.0, 175.0), 2.0),
        HueRange::new((180.0, 200.0), 1.0),
        HueRange::new((0.0, 15.0), 1.0),
    ];
    (0..count)
        .map(|_| {
            let roll: f64 = rng.gen();
            if roll < 0.25 {
                // Gold
                let h = random_in_range(rng, 85.0, 100.0);
                oklch_to_colour(
                    random_in_range(rng, 0.7, 0.8),
                    random_in_range(rng, 0.12, 0.18),
                    h,
                )
            } else if roll < 0.4 {
                // Black
                let h = random_in_range(rng, 0.0, 360.0);
                oklch_to_colour(
                    random_in_range(rng, 0.12, 0.2),
                    random_in_range(rng, 0.01, 0.03),
                    h,
                )
            } else if roll < 0.55 {
                // Cream
                let h = random_in_range(rng, 80.0, 100.0);
                oklch_to_colour(
                    random_in_range(rng, 0.9, 0.96),
                    random_in_range(rng, 0.015, 0.03),
                    h,
                )
            } else {
                pick_from_hue_ranges(rng, &jewel, (0.35, 0.55), (0.1, 0.18))
            }
        })
        .collect()
}

fn japanese(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let traditional = [
        HueRange::with_lc((245.0, 270.0), 3.0, (0.25, 0.45), (0.06, 0.14)),
        HueRange::with_lc((18.0, 35.0), 2.0, (0.45, 0.58), (0.14, 0.22)),
        HueRange::with_lc((0.0, 18.0), 1.0, (0.35, 0.48), (0.12, 0.18)),
        HueRange::with_lc((75.0, 95.0), 2.0, (0.7, 0.82), (0.1, 0.16)),
        HueRange::with_lc((120.0, 145.0), 2.0, (0.35, 0.5), (0.06, 0.12)),
        HueRange::with_lc((290.0, 320.0), 1.0, (0.5, 0.7), (0.08, 0.14)),
        HueRange::with_lc((340.0, 360.0), 1.0, (0.75, 0.88), (0.06, 0.12)),
        HueRange::with_lc((35.0, 50.0), 1.0, (0.55, 0.68), (0.12, 0.18)),
    ];
    (0..count)
        .map(|_| {
            let roll: f64 = rng.gen();
            if roll < 0.15 {
                // Pale neutrals
                let h = random_in_range(rng, 70.0, 100.0);
                oklch_to_colour(
                    random_in_range(rng, 0.88, 0.95),
                    random_in_range(rng, 0.01, 0.03),
                    h,
                )
            } else if roll < 0.25 {
                // Earth tones
                let h = random_in_range(rng, 35.0, 60.0);
                oklch_to_colour(
                    random_in_range(rng, 0.4, 0.55),
                    random_in_range(rng, 0.05, 0.1),
                    h,
                )
            } else {
                pick_from_hue_ranges(rng, &traditional, (0.4, 0.6), (0.08, 0.15))
            }
        })
        .collect()
}

fn scandinavian(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    (0..count)
        .map(|_| {
            let roll: f64 = rng.gen();
            if roll < 0.35 {
                // White / off-white
                let h = random_in_range(rng, 80.0, 110.0);
                oklch_to_colour(
                    random_in_range(rng, 0.93, 0.98),
                    random_in_range(rng, 0.005, 0.015),
                    h,
                )
            } else if roll < 0.55 {
                // Pale grey
                let h = random_in_range(rng, 200.0, 260.0);
                oklch_to_colour(
                    random_in_range(rng, 0.8, 0.9),
                    random_in_range(rng, 0.005, 0.015),
                    h,
                )
            } else if roll < 0.75 {
                // Muted pastel
                let h = random_in_range(rng, 0.0, 360.0);
                oklch_to_colour(
                    random_in_range(rng, 0.8, 0.9),
                    random_in_range(rng, 0.02, 0.05),
                    h,
                )
            } else {
                // Wood tone
                let h = random_in_range(rng, 50.0, 80.0);
                oklch_to_colour(
                    random_in_range(rng, 0.55, 0.7),
                    random_in_range(rng, 0.04, 0.08),
                    h,
                )
            }
        })
        .collect()
}

fn mexican(count: usize, rng: &mut dyn RngCore) -> Vec<Colour> {
    let ranges = [
        HueRange::new((330.0, 350.0), 2.0),
        HueRange::new((20.0, 40.0), 2.0),
        HueRange::new((175.0, 195.0), 2.0),
        HueRange::new((55.0, 70.0), 2.0),
        HueRange::new((280.0, 310.0), 1.0),
    ];
    (0..count)
        .map(|_| pick_from_hue_ranges(rng, &ranges, (0.55, 0.72), (0.18, 0.28)))
        .collect()
}

// ============================================================================
// LOCK PARSING
// ============================================================================

/// Parse a lock string like `"0:#ff6600,3:#003366"` into (index, colour) pairs.
fn parse_locks(input: &str, size: usize) -> Result<Vec<(usize, Colour)>, Error> {
    let mut out = Vec::new();
    for item in input.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (idx_part, colour_part) = item.split_once(':').ok_or_else(|| {
            Error::Usage(format!(
                "invalid lock entry: {item}\nexpected format: <index>:<colour> (e.g. 0:#ff6600)"
            ))
        })?;
        let idx: usize = idx_part.trim().parse().map_err(|_| {
            Error::Usage(format!("invalid lock index: {idx_part}"))
        })?;
        if idx >= size {
            return Err(Error::Usage(format!(
                "lock index {idx} is out of range for palette size {size}"
            )));
        }
        let colour = Colour::parse(colour_part.trim())?;
        out.push((idx, colour));
    }
    Ok(out)
}

// ============================================================================
// OUTPUT
// ============================================================================

fn print_list() {
    for &cat in Category::order() {
        println!("{}", cat.title());
        for &strat in Strategy::all() {
            if strat.category() == cat {
                let (slug, name, desc, _) = strat.info();
                println!("  {:<20} {} — {}", slug, name, desc);
            }
        }
        println!();
    }
}

fn output_hex(colours: &[Colour], with_index: bool) {
    let idx_width = (colours.len().saturating_sub(1).to_string()).len();
    for (i, c) in colours.iter().enumerate() {
        let (r, g, b) = c.to_u8();
        let block = output::colour_block(r, g, b);
        let prefix = if with_index {
            format!("[{i:>idx_width$}] ")
        } else {
            String::new()
        };
        if block.is_empty() {
            println!("{prefix}{}", c.to_hex());
        } else {
            println!("{prefix}{block}  {}", c.to_hex());
        }
    }
}

fn output_css(colours: &[Colour]) {
    for (i, c) in colours.iter().enumerate() {
        let (r, g, b) = c.to_u8();
        let block = output::colour_block(r, g, b);
        if block.is_empty() {
            println!("--palette-{}: {};", i + 1, c.to_hex());
        } else {
            println!("{block}  --palette-{}: {};", i + 1, c.to_hex());
        }
    }
}

fn output_json(colours: &[Colour]) {
    let entries: Vec<serde_json::Value> = colours
        .iter()
        .map(|c| {
            let (r, g, b) = c.to_u8();
            let (lab_l, lab_a, lab_b) = crate::colour::srgb_to_oklab(c.r, c.g, c.b);
            let (l, ch, h) = crate::colour::oklab_to_oklch(lab_l, lab_a, lab_b);
            json!({
                "hex": c.to_hex(),
                "rgb": [r, g, b],
                "oklch": [
                    (l * 10000.0).round() / 10000.0,
                    (ch * 10000.0).round() / 10000.0,
                    (h * 100.0).round() / 100.0,
                ],
            })
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&json!(entries)).unwrap());
}

fn output_png(colours: &[Colour], path: &Path) -> Result<(), Error> {
    const SWATCH: u32 = 100;
    let n = colours.len() as u32;
    if n == 0 {
        return Err(Error::Processing("palette is empty; cannot render PNG".into()));
    }
    let width = SWATCH * n;
    let height = SWATCH;
    let img = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::from_fn(width, height, |x, _y| {
        let idx = (x / SWATCH) as usize;
        let c = &colours[idx.min(colours.len() - 1)];
        let (r, g, b) = c.to_u8();
        image::Rgb([r, g, b])
    });
    img.save(path)
        .map_err(|e| Error::Processing(format!("failed to write PNG: {e}")))?;
    Ok(())
}

// ============================================================================
// ENTRY POINT
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub fn run(
    strategy: Option<&str>,
    size: usize,
    format: &str,
    lock: Option<&str>,
    seed: Option<u64>,
    pretty: bool,
    list: bool,
    json_global: bool,
    output_path: Option<&Path>,
) -> Result<(), Error> {
    if list {
        print_list();
        return Ok(());
    }

    // Default strategy: random-cohesive — gives a usable palette on a bare
    // `delphi palette` invocation. Use `--list` to see the full menu.
    let strat = match strategy {
        Some(s) => Strategy::parse(s)?,
        None => Strategy::RandomCohesive,
    };

    if size == 0 {
        return Err(Error::Usage("--size must be at least 1".into()));
    }

    // Determine output format: the global --json flag wins, otherwise --format value.
    let effective_format = if json_global { "json" } else { format };
    match effective_format {
        "hex" | "css" | "json" | "png" => {}
        other => {
            return Err(Error::Usage(format!(
                "unknown format: {other}\nvalid formats: hex, css, json, png"
            )))
        }
    }

    // Generate (then overwrite locked slots so seed determinism is preserved).
    let mut rng = build_rng(seed);
    let mut colours = strat.generate(size, &mut rng);

    if let Some(lock_str) = lock {
        for (idx, c) in parse_locks(lock_str, size)? {
            colours[idx] = c;
        }
    }

    match effective_format {
        "hex" => output_hex(&colours, pretty),
        "css" => output_css(&colours),
        "json" => output_json(&colours),
        "png" => {
            let default_path = Path::new("palette.png");
            let path = output_path.unwrap_or(default_path);
            output_png(&colours, path)?;
            println!("{}", path.display());
        }
        _ => unreachable!(),
    }

    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rng(seed: u64) -> AnyRng {
        AnyRng::Seeded(ChaCha8Rng::seed_from_u64(seed))
    }

    // --- Strategy parsing ---

    #[test]
    fn parse_known_strategies() {
        for &strat in Strategy::all() {
            let slug = strat.info().0;
            assert_eq!(Strategy::parse(slug).unwrap(), strat);
        }
    }

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(Strategy::parse("ANALOGOUS").unwrap(), Strategy::Analogous);
        assert_eq!(Strategy::parse("80s").unwrap(), Strategy::Eighties);
    }

    #[test]
    fn parse_unknown_strategy() {
        assert!(Strategy::parse("rainbow").is_err());
        assert!(Strategy::parse("").is_err());
    }

    // --- Size honoured ---

    #[test]
    fn size_honoured_for_all_strategies() {
        for &strat in Strategy::all() {
            let mut rng = make_rng(42);
            let colours = strat.generate(5, &mut rng);
            assert_eq!(colours.len(), 5, "strategy {:?} produced wrong size", strat);
        }
    }

    #[test]
    fn size_one_works() {
        // Strategies that divide by (count-1) should still cope with size=1.
        for &strat in &[Strategy::Analogous, Strategy::Monochromatic] {
            let mut rng = make_rng(1);
            let colours = strat.generate(1, &mut rng);
            assert_eq!(colours.len(), 1, "strategy {:?} failed at size 1", strat);
        }
    }

    #[test]
    fn size_large() {
        let mut rng = make_rng(1);
        let colours = Strategy::Triadic.generate(20, &mut rng);
        assert_eq!(colours.len(), 20);
    }

    // --- Seed determinism ---

    #[test]
    fn seed_is_deterministic() {
        let mut a = make_rng(123);
        let mut b = make_rng(123);
        let pa = Strategy::Eighties.generate(8, &mut a);
        let pb = Strategy::Eighties.generate(8, &mut b);
        let ha: Vec<String> = pa.iter().map(|c| c.to_hex()).collect();
        let hb: Vec<String> = pb.iter().map(|c| c.to_hex()).collect();
        assert_eq!(ha, hb);
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = make_rng(1);
        let mut b = make_rng(2);
        let pa = Strategy::Eighties.generate(8, &mut a);
        let pb = Strategy::Eighties.generate(8, &mut b);
        let ha: Vec<String> = pa.iter().map(|c| c.to_hex()).collect();
        let hb: Vec<String> = pb.iter().map(|c| c.to_hex()).collect();
        assert_ne!(ha, hb);
    }

    // --- Wrap-range gotcha (no panic on `[350, 20]` etc.) ---

    #[test]
    fn wrap_range_strategies_dont_panic() {
        for seed in 0..16u64 {
            let mut rng = make_rng(seed);
            let _ = Strategy::Nineties.generate(12, &mut rng);
            let mut rng = make_rng(seed);
            let _ = Strategy::DesertDusk.generate(12, &mut rng);
        }
    }

    // --- Lock parsing & application ---

    #[test]
    fn parse_locks_single() {
        let locks = parse_locks("0:#ff6600", 5).unwrap();
        assert_eq!(locks.len(), 1);
        assert_eq!(locks[0].0, 0);
        assert_eq!(locks[0].1.to_hex(), "#ff6600");
    }

    #[test]
    fn parse_locks_multi() {
        let locks = parse_locks("0:#ff6600,3:#003366", 5).unwrap();
        assert_eq!(locks.len(), 2);
        assert_eq!(locks[1].0, 3);
        assert_eq!(locks[1].1.to_hex(), "#003366");
    }

    #[test]
    fn parse_locks_out_of_range() {
        assert!(parse_locks("9:#ff0000", 5).is_err());
    }

    #[test]
    fn parse_locks_bad_format() {
        assert!(parse_locks("garbage", 5).is_err());
    }

    #[test]
    fn locks_applied_via_run() {
        // Use png format with a temp path, since stdout outputs are harder to capture;
        // instead, just confirm the colours vector logic via direct call.
        let mut rng = make_rng(7);
        let mut colours = Strategy::Analogous.generate(5, &mut rng);
        for (idx, c) in parse_locks("0:#ff6600,3:#003366", 5).unwrap() {
            colours[idx] = c;
        }
        assert_eq!(colours[0].to_hex(), "#ff6600");
        assert_eq!(colours[3].to_hex(), "#003366");
        assert_eq!(colours.len(), 5);
    }

    // --- Format dispatch (smoke test) ---

    #[test]
    fn run_format_hex() {
        assert!(run(Some("analogous"), 4, "hex", None, Some(1), false, false, false, None).is_ok());
    }

    #[test]
    fn run_format_css() {
        assert!(run(Some("analogous"), 4, "css", None, Some(1), false, false, false, None).is_ok());
    }

    #[test]
    fn run_format_json() {
        assert!(run(Some("analogous"), 4, "json", None, Some(1), false, false, false, None).is_ok());
    }

    #[test]
    fn run_format_png() {
        let dir = std::env::temp_dir();
        let path = dir.join("delphi_palette_test.png");
        let result = run(
            Some("analogous"),
            4,
            "png",
            None,
            Some(1),
            false,
            false,
            false,
            Some(&path),
        );
        assert!(result.is_ok());
        assert!(path.exists());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn run_global_json_overrides_format() {
        assert!(run(Some("analogous"), 4, "hex", None, Some(1), false, false, true, None).is_ok());
    }

    #[test]
    fn run_list_short_circuits() {
        // --list should succeed even with no strategy given.
        assert!(run(None, 5, "hex", None, None, false, true, false, None).is_ok());
    }

    #[test]
    fn run_missing_strategy_defaults_to_random_cohesive() {
        // Bare `delphi palette` should succeed using the random-cohesive default.
        assert!(run(None, 5, "hex", None, Some(1), false, false, false, None).is_ok());
    }

    #[test]
    fn run_unknown_format() {
        assert!(run(Some("analogous"), 4, "yaml", None, Some(1), false, false, false, None).is_err());
    }

    #[test]
    fn run_size_zero_errors() {
        assert!(run(Some("analogous"), 0, "hex", None, Some(1), false, false, false, None).is_err());
    }

    #[test]
    fn run_with_locks() {
        assert!(run(
            Some("analogous"),
            5,
            "hex",
            Some("0:#ff6600,3:#003366"),
            Some(1),
            false,
            false,
            false,
            None,
        )
        .is_ok());
    }
}
