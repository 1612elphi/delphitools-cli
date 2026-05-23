use crate::colour::Colour;
use crate::error::Error;
use crate::image_tools::{derive_output, open_image, parse_ratio};
use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};
use imageproc::filter::gaussian_blur_f32;
use serde_json::json;
use std::path::{Path, PathBuf};

pub fn run(
    image: &Path,
    ratio: &str,
    fill: &str,
    colour: &str,
    json_out: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    let (rw, rh) = parse_ratio(ratio)?;
    let fill_mode = parse_fill(fill)?;
    let fill_colour = if matches!(fill_mode, FillMode::Colour) {
        Some(Colour::parse(colour)?)
    } else {
        None
    };

    let img = open_image(image)?;

    let (iw, ih) = (img.width(), img.height());
    let tile_h = ih;
    let tile_w = ((tile_h as f64) * (rw / rh)).round().max(1.0) as u32;

    let n_tiles = ((iw as f64) / (tile_w as f64)).ceil().max(1.0) as u32;
    let total_w = tile_w * n_tiles;
    let pad_total = total_w.saturating_sub(iw);
    let pad_left = pad_total / 2;

    let mut tile_paths: Vec<PathBuf> = Vec::with_capacity(n_tiles as usize);

    // Scroll always produces multiple tiles, so --output must be (or become) a directory.
    let (out_dir, base_stem): (PathBuf, String) = match output {
        Some(p) => {
            if p.exists() && !p.is_dir() {
                return Err(Error::Usage(format!(
                    "--output must be a directory for scroll (tiles are emitted as separate files): {}",
                    p.display()
                )));
            }
            std::fs::create_dir_all(p).map_err(|e| {
                Error::Processing(format!("could not create {}: {e}", p.display()))
            })?;
            (
                p.to_path_buf(),
                image.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or("image".into()),
            )
        }
        None => {
            let derived = derive_output(image, "tile-1", Some("png"));
            (
                derived.parent().unwrap_or_else(|| Path::new(".")).to_path_buf(),
                image.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or("image".into()),
            )
        }
    };

    for i in 0..n_tiles {
        let tile = build_tile(
            &img, iw, tile_w, tile_h, pad_left, total_w, i, n_tiles, fill_mode, fill_colour,
        );
        let tile_path = out_dir.join(format!("{base_stem}-tile-{}.png", i + 1));
        tile.save(&tile_path).map_err(|e| {
            Error::Processing(format!("could not save {}: {e}", tile_path.display()))
        })?;
        tile_paths.push(tile_path);
    }

    if json_out {
        let arr: Vec<String> = tile_paths.iter().map(|p| p.display().to_string()).collect();
        let obj = json!({
            "input": image.display().to_string(),
            "outputs": arr,
            "tiles": n_tiles,
            "tile_width": tile_w,
            "tile_height": tile_h,
        });
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
    } else if !quiet {
        for p in &tile_paths {
            println!("{} -> {}", image.display(), p.display());
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum FillMode {
    Blur,
    Colour,
}

fn parse_fill(s: &str) -> Result<FillMode, Error> {
    match s.to_ascii_lowercase().as_str() {
        "blur" | "blurred" => Ok(FillMode::Blur),
        "colour" | "color" | "solid" => Ok(FillMode::Colour),
        other => Err(Error::Usage(format!("invalid fill mode: {other}"))),
    }
}

#[allow(clippy::too_many_arguments)]
fn build_tile(
    img: &DynamicImage,
    iw: u32,
    tile_w: u32,
    tile_h: u32,
    pad_left: u32,
    total_w: u32,
    index: u32,
    n_tiles: u32,
    mode: FillMode,
    fill: Option<Colour>,
) -> RgbaImage {
    let mut canvas = RgbaImage::new(tile_w, tile_h);

    let tile_start = index * tile_w; // in virtual (padded) coords
    let tile_end = tile_start + tile_w;
    let image_start = pad_left;
    let image_end = pad_left + iw;
    let overlap_start = tile_start.max(image_start);
    let overlap_end = tile_end.min(image_end);
    let needs_fill = total_w > iw && (index == 0 || index == n_tiles - 1);

    if needs_fill {
        match mode {
            FillMode::Colour => {
                let c = fill.unwrap_or_else(|| Colour::from_u8(255, 255, 255));
                let (r, g, b) = c.to_u8();
                let a = (c.a.clamp(0.0, 1.0) * 255.0).round() as u8;
                for p in canvas.pixels_mut() {
                    p.0 = [r, g, b, a];
                }
            }
            FillMode::Blur => {
                // Take what part of the image overlaps this tile and stretch+blur to fill.
                if overlap_end > overlap_start {
                    let src_x = overlap_start - image_start;
                    let src_w = overlap_end - overlap_start;
                    let slice = img.crop_imm(src_x, 0, src_w, img.height());
                    let scale = (tile_w as f32 / src_w as f32)
                        .max(tile_h as f32 / img.height() as f32)
                        * 1.2;
                    let sw = ((src_w as f32 * scale).round() as u32).max(1);
                    let sh = ((img.height() as f32 * scale).round() as u32).max(1);
                    let scaled = slice.resize_exact(sw, sh, FilterType::Triangle).to_rgba8();
                    let sigma = ((tile_w.max(tile_h) as f32) * 0.04).max(8.0);
                    let blurred = gaussian_blur_f32(&scaled, sigma);
                    let off_x = sw.saturating_sub(tile_w) / 2;
                    let off_y = sh.saturating_sub(tile_h) / 2;
                    for y in 0..tile_h {
                        for x in 0..tile_w {
                            let sx = (off_x + x).min(sw - 1);
                            let sy = (off_y + y).min(sh - 1);
                            canvas.put_pixel(x, y, *blurred.get_pixel(sx, sy));
                        }
                    }
                } else {
                    // No image content — fall back to black.
                    for p in canvas.pixels_mut() {
                        p.0 = [0, 0, 0, 255];
                    }
                }
            }
        }
    }

    // Stamp the real image content for the overlap region.
    if overlap_end > overlap_start {
        let draw_x = overlap_start - tile_start;
        let src_x = overlap_start - image_start;
        let w = overlap_end - overlap_start;
        let slice = img.crop_imm(src_x, 0, w, img.height()).to_rgba8();
        for y in 0..tile_h.min(slice.height()) {
            for x in 0..w {
                let p = *slice.get_pixel(x, y);
                let dx = draw_x + x;
                if dx < tile_w {
                    canvas.put_pixel(dx, y, p);
                }
            }
        }
    }

    canvas
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use std::fs;

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("delphi-scroll-test-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_image(path: &Path, w: u32, h: u32) {
        let mut img = RgbaImage::new(w, h);
        for p in img.pixels_mut() {
            p.0 = [200, 100, 50, 255];
        }
        img.save(path).unwrap();
    }

    #[test]
    fn perfect_fit() {
        // 800x1000 with tile 4:5 → tile_w=800, n=1
        let dir = tmpdir("a");
        let input = dir.join("in.png");
        make_image(&input, 800, 1000);
        run(&input, "4:5", "blur", "#ffffff", false, true, Some(&dir)).unwrap();
        assert!(dir.join("in-tile-1.png").exists());
        assert!(!dir.join("in-tile-2.png").exists());
    }

    #[test]
    fn three_tiles_with_blur_fill() {
        // 2500x1000 with 4:5 → tile_w=800; ceil(2500/800) = 4 tiles
        let dir = tmpdir("b");
        let input = dir.join("in.png");
        make_image(&input, 2500, 1000);
        run(&input, "4:5", "blur", "#ffffff", false, true, Some(&dir)).unwrap();
        for i in 1..=4 {
            assert!(dir.join(format!("in-tile-{i}.png")).exists(), "tile-{i} missing");
        }
        // Tile dimensions should be 800x1000
        let t = image::open(dir.join("in-tile-1.png")).unwrap();
        assert_eq!(t.width(), 800);
        assert_eq!(t.height(), 1000);
    }

    #[test]
    fn solid_fill_uses_colour() {
        let dir = tmpdir("c");
        let input = dir.join("in.png");
        make_image(&input, 1500, 1000);
        run(&input, "4:5", "colour", "#00ff00", false, true, Some(&dir)).unwrap();
        // First tile's leftmost column should be the fill colour.
        let img = image::open(dir.join("in-tile-1.png")).unwrap().to_rgba8();
        let p = img.get_pixel(0, 0).0;
        assert_eq!(p[1], 255);
        assert_eq!(p[0], 0);
    }
}
