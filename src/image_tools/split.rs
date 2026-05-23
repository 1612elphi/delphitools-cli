use crate::error::Error;
use crate::image_tools::{derive_output, open_image};
use serde_json::json;
use std::path::{Path, PathBuf};

pub fn run(
    image: &Path,
    rows: u32,
    cols: u32,
    json_out: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if rows == 0 || cols == 0 {
        return Err(Error::Usage("rows and cols must be > 0".into()));
    }
    if !image.exists() {
        return Err(Error::Input(format!("file not found: {}", image.display())));
    }

    let img = open_image(image)?;

    let (iw, ih) = (img.width(), img.height());
    let tile_w = iw / cols;
    let tile_h = ih / rows;
    if tile_w == 0 || tile_h == 0 {
        return Err(Error::Usage(format!(
            "image {iw}x{ih} is too small for a {rows}x{cols} split"
        )));
    }

    let ext = image
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "png".to_string());

    let (out_dir, stem) = match output {
        Some(p) => {
            std::fs::create_dir_all(p)
                .map_err(|e| Error::Processing(format!("could not create {}: {e}", p.display())))?;
            let stem = image
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "image".to_string());
            (p.to_path_buf(), stem)
        }
        None => {
            let derived = derive_output(image, "tile-1-1", Some(&ext));
            let parent = derived
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();
            let stem = image
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "image".to_string());
            (parent, stem)
        }
    };

    let mut paths: Vec<PathBuf> = Vec::with_capacity((rows * cols) as usize);
    let mut img = img;
    for r in 0..rows {
        for c in 0..cols {
            let tile = img.crop(c * tile_w, r * tile_h, tile_w, tile_h);
            let path = out_dir.join(format!("{stem}-tile-{}-{}.{ext}", r + 1, c + 1));
            tile.save(&path).map_err(|e| {
                Error::Processing(format!("could not save {}: {e}", path.display()))
            })?;
            paths.push(path);
        }
    }

    if json_out {
        let obj = json!({
            "input": image.display().to_string(),
            "outputs": paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "rows": rows,
            "cols": cols,
            "tile_width": tile_w,
            "tile_height": tile_h,
        });
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
    } else if !quiet {
        for p in &paths {
            println!("{} -> {}", image.display(), p.display());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use std::fs;

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("delphi-split-test-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_image(path: &Path, w: u32, h: u32) {
        let mut img = RgbaImage::new(w, h);
        for p in img.pixels_mut() {
            p.0 = [0, 128, 255, 255];
        }
        img.save(path).unwrap();
    }

    #[test]
    fn split_grid() {
        let dir = tmpdir("a");
        let input = dir.join("in.png");
        make_image(&input, 300, 200);
        run(&input, 2, 3, false, true, Some(&dir)).unwrap();
        for r in 1..=2 {
            for c in 1..=3 {
                let path = dir.join(format!("in-tile-{r}-{c}.png"));
                assert!(path.exists(), "{} missing", path.display());
                let p = image::open(&path).unwrap();
                assert_eq!(p.width(), 100);
                assert_eq!(p.height(), 100);
            }
        }
    }

    #[test]
    fn rejects_zero() {
        let dir = tmpdir("b");
        let input = dir.join("in.png");
        make_image(&input, 100, 100);
        assert!(run(&input, 0, 1, false, true, None).is_err());
    }
}
