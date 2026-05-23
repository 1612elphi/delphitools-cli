use crate::error::Error;
use crate::image_tools::{derive_output, open_image, resolve_output, run_batch, save_rgba, BatchOk};
use image::RgbaImage;
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::path::{Path, PathBuf};

pub fn run(
    images: &[PathBuf],
    opacity: f32,
    scale: f32,
    seed: Option<u64>,
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if !(0.0..=1.0).contains(&opacity) {
        return Err(Error::Usage(format!(
            "opacity must be between 0 and 1, got {opacity}"
        )));
    }
    if !(1.0..=16.0).contains(&scale) {
        return Err(Error::Usage(format!(
            "scale must be between 1.0 and 16.0, got {scale}"
        )));
    }
    let n = images.len();

    run_batch(images, json, quiet, |input| {
        let img = open_image(input)?.to_rgba8();

        // Build a separate RNG per file when no seed → fresh entropy; when seeded,
        // derive a deterministic per-file seed by mixing input path bytes.
        let mut rng: Box<dyn RngCore> = match seed {
            Some(s) => {
                let mut h: u64 = s;
                for &b in input.to_string_lossy().as_bytes() {
                    h = h.wrapping_mul(0x100000001b3).wrapping_add(b as u64);
                }
                Box::new(ChaCha8Rng::seed_from_u64(h))
            }
            None => Box::new(rand::thread_rng()),
        };

        let out = apply_noise(&img, opacity, scale, rng.as_mut());

        let derived = derive_output(input, "noise", None);
        let out_path = resolve_output(output, n, &derived)?;
        // Preserve the original alpha; save based on output extension.
        save_rgba(&out, &out_path)?;
        Ok(BatchOk::one(out_path))
    })
}

fn apply_noise(img: &RgbaImage, opacity: f32, scale: f32, rng: &mut dyn RngCore) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut out = RgbaImage::new(w, h);

    // Generate noise at a downscaled resolution if scale > 1 → block-style noise.
    let nw = ((w as f32) / scale).ceil().max(1.0) as u32;
    let nh = ((h as f32) / scale).ceil().max(1.0) as u32;
    let noise_count = (nw * nh) as usize;
    let mut noise: Vec<[u8; 3]> = Vec::with_capacity(noise_count);
    for _ in 0..noise_count {
        noise.push([
            rng.gen_range(0..=255),
            rng.gen_range(0..=255),
            rng.gen_range(0..=255),
        ]);
    }

    for y in 0..h {
        for x in 0..w {
            let nx = ((x as f32) / scale).floor() as u32;
            let ny = ((y as f32) / scale).floor() as u32;
            let nx = nx.min(nw - 1);
            let ny = ny.min(nh - 1);
            let n = noise[(ny * nw + nx) as usize];
            let src = img.get_pixel(x, y).0;
            // Overlay blend per channel, then mix by `opacity`.
            let r = overlay(src[0], n[0]);
            let g = overlay(src[1], n[1]);
            let b = overlay(src[2], n[2]);
            let mr = mix(src[0], r, opacity);
            let mg = mix(src[1], g, opacity);
            let mb = mix(src[2], b, opacity);
            out.put_pixel(x, y, image::Rgba([mr, mg, mb, src[3]]));
        }
    }
    out
}

#[inline]
fn overlay(base: u8, top: u8) -> u8 {
    let b = base as f32 / 255.0;
    let t = top as f32 / 255.0;
    let r = if b <= 0.5 {
        2.0 * b * t
    } else {
        1.0 - 2.0 * (1.0 - b) * (1.0 - t)
    };
    (r.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[inline]
fn mix(orig: u8, blended: u8, opacity: f32) -> u8 {
    let o = opacity.clamp(0.0, 1.0);
    let r = orig as f32 * (1.0 - o) + blended as f32 * o;
    r.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use std::fs;

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("delphi-noise-test-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_image(path: &Path, w: u32, h: u32) {
        let mut img = RgbaImage::new(w, h);
        for p in img.pixels_mut() {
            p.0 = [128, 128, 128, 255];
        }
        img.save(path).unwrap();
    }

    #[test]
    fn noise_deterministic_with_seed() {
        let dir = tmpdir("a");
        let input = dir.join("in.png");
        make_image(&input, 64, 64);

        let out1 = dir.join("a.png");
        let out2 = dir.join("b.png");
        run(&[input.clone()], 0.3, 1.0, Some(42), false, true, Some(&out1)).unwrap();
        run(&[input.clone()], 0.3, 1.0, Some(42), false, true, Some(&out2)).unwrap();

        let i1 = image::open(&out1).unwrap().to_rgba8();
        let i2 = image::open(&out2).unwrap().to_rgba8();
        assert_eq!(i1.as_raw(), i2.as_raw(), "same seed → identical output");
    }

    #[test]
    fn overlay_identity_at_grey() {
        // Overlay with neutral grey leaves grey alone.
        assert_eq!(overlay(128, 128), 128);
    }

    #[test]
    fn overlay_extremes() {
        // Black base + anything = black.
        assert_eq!(overlay(0, 200), 0);
        // White base + anything = white.
        assert_eq!(overlay(255, 50), 255);
    }

    #[test]
    fn mix_zero_opacity_keeps_original() {
        assert_eq!(mix(100, 200, 0.0), 100);
        assert_eq!(mix(100, 200, 1.0), 200);
    }

    #[test]
    fn rejects_bad_opacity() {
        let r = run(&[], 2.0, 1.0, None, false, true, None);
        assert!(r.is_err());
    }
}
