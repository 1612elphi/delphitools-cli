use crate::error::Error;
use crate::image_tools::open_image;
use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};
use serde_json::json;
use std::path::{Path, PathBuf};

pub fn run(
    image: &Path,
    sizes: &str,
    ico: bool,
    json_out: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if !image.exists() {
        return Err(Error::Input(format!("file not found: {}", image.display())));
    }
    let sizes = parse_sizes(sizes)?;

    let mut img = open_image(image)?;
    let square = crop_to_square(&mut img);

    let out_dir: PathBuf = match output {
        Some(p) => {
            std::fs::create_dir_all(p).map_err(|e| {
                Error::Processing(format!("could not create {}: {e}", p.display()))
            })?;
            p.to_path_buf()
        }
        None => image.parent().unwrap_or_else(|| Path::new(".")).to_path_buf(),
    };
    let stem = image
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "favicon".to_string());

    // Generate PNGs and keep RGBA buffers around for ICO assembly.
    let mut written: Vec<PathBuf> = Vec::with_capacity(sizes.len());
    let mut rgba_for_ico: Vec<(u32, RgbaImage)> = Vec::new();
    for &sz in &sizes {
        let resized = square.resize_exact(sz, sz, FilterType::Lanczos3).to_rgba8();
        let out_path = out_dir.join(format!("{stem}-{sz}x{sz}.png"));
        resized
            .save(&out_path)
            .map_err(|e| Error::Processing(format!("could not save {}: {e}", out_path.display())))?;
        written.push(out_path);
        if ico && sz <= 256 {
            rgba_for_ico.push((sz, resized));
        }
    }

    let mut ico_path: Option<PathBuf> = None;
    if ico {
        if rgba_for_ico.is_empty() {
            return Err(Error::Usage(
                "no sizes ≤ 256 for ICO output (ICO supports up to 256x256)".into(),
            ));
        }
        let path = out_dir.join("favicon.ico");
        write_ico(&rgba_for_ico, &path)?;
        ico_path = Some(path);
    }

    if json_out {
        let mut obj = serde_json::Map::new();
        obj.insert("input".into(), json!(image.display().to_string()));
        obj.insert(
            "outputs".into(),
            json!(written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>()),
        );
        obj.insert("sizes".into(), json!(sizes));
        if let Some(ref p) = ico_path {
            obj.insert("ico".into(), json!(p.display().to_string()));
        }
        println!("{}", serde_json::to_string_pretty(&serde_json::Value::Object(obj)).unwrap());
    } else if !quiet {
        for p in &written {
            println!("{} -> {}", image.display(), p.display());
        }
        if let Some(ref p) = ico_path {
            println!("{} -> {}", image.display(), p.display());
        }
    }

    Ok(())
}

fn parse_sizes(s: &str) -> Result<Vec<u32>, Error> {
    let mut out: Vec<u32> = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let n: u32 = part
            .parse()
            .map_err(|_| Error::Usage(format!("invalid size: {part}")))?;
        if n == 0 || n > 4096 {
            return Err(Error::Usage(format!("size out of range: {part}")));
        }
        out.push(n);
    }
    if out.is_empty() {
        return Err(Error::Usage("no sizes given".into()));
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

fn crop_to_square(img: &mut DynamicImage) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w == h {
        return img.clone();
    }
    let side = w.min(h);
    let x = (w - side) / 2;
    let y = (h - side) / 2;
    img.crop(x, y, side, side)
}

fn write_ico(rgba: &[(u32, RgbaImage)], path: &Path) -> Result<(), Error> {
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for (sz, img) in rgba {
        let icon = ico::IconImage::from_rgba_data(*sz, *sz, img.as_raw().clone());
        let entry = ico::IconDirEntry::encode(&icon)
            .map_err(|e| Error::Processing(format!("ICO encode failed at {sz}: {e}")))?;
        dir.add_entry(entry);
    }
    let f = std::fs::File::create(path)
        .map_err(|e| Error::Processing(format!("could not create {}: {e}", path.display())))?;
    dir.write(f)
        .map_err(|e| Error::Processing(format!("could not write ico: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use std::fs;

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("delphi-fav-test-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_image(path: &Path, w: u32, h: u32) {
        let mut img = RgbaImage::new(w, h);
        for p in img.pixels_mut() {
            p.0 = [10, 200, 100, 255];
        }
        img.save(path).unwrap();
    }

    #[test]
    fn parse_sizes_basic() {
        assert_eq!(parse_sizes("16,32,180").unwrap(), vec![16, 32, 180]);
        assert_eq!(parse_sizes(" 16 ,16,32 ").unwrap(), vec![16, 32]);
        assert!(parse_sizes("").is_err());
        assert!(parse_sizes("0").is_err());
        assert!(parse_sizes("abc").is_err());
    }

    #[test]
    fn favicon_writes_pngs() {
        let dir = tmpdir("a");
        let input = dir.join("logo.png");
        make_image(&input, 300, 500);
        run(&input, "16,32", false, false, true, Some(&dir)).unwrap();
        assert!(dir.join("logo-16x16.png").exists());
        assert!(dir.join("logo-32x32.png").exists());
        let p = image::open(dir.join("logo-32x32.png")).unwrap();
        assert_eq!(p.width(), 32);
        assert_eq!(p.height(), 32);
    }

    #[test]
    fn favicon_writes_ico() {
        let dir = tmpdir("b");
        let input = dir.join("logo.png");
        make_image(&input, 300, 300);
        run(&input, "16,32,48", true, false, true, Some(&dir)).unwrap();
        assert!(dir.join("favicon.ico").exists());
    }
}
