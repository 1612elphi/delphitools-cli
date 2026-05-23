pub mod clip;
pub mod convert;
pub mod crop;
pub mod favicon;
pub mod matte;
pub mod noise;
pub mod rmbg;
pub mod scroll;
pub mod split;
pub mod svgo;
pub mod trace;
pub mod watermark;

use crate::error::Error;
use image::{DynamicImage, ImageReader, RgbaImage};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Open and fully decode an image at `path`.
/// Collapses the standard open + with_guessed_format + decode chain into one call
/// with consistent error messages.
pub fn open_image(path: &Path) -> Result<DynamicImage, Error> {
    ImageReader::open(path)
        .map_err(|e| Error::Input(format!("could not open {}: {e}", path.display())))?
        .with_guessed_format()
        .map_err(|e| Error::Input(format!("could not read {}: {e}", path.display())))?
        .decode()
        .map_err(|e| Error::Input(format!("could not decode {}: {e}", path.display())))
}

/// Save an `RgbaImage`, flattening alpha onto white when the target format
/// (jpg/jpeg/bmp) does not support transparency.
pub fn save_rgba(img: &RgbaImage, path: &Path) -> Result<(), Error> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let save = |save_result: image::ImageResult<()>| {
        save_result
            .map_err(|e| Error::Processing(format!("could not save {}: {e}", path.display())))
    };
    match ext.as_str() {
        "jpg" | "jpeg" | "bmp" => save(flatten_alpha_to_rgb(img).save(path)),
        _ => save(img.save(path)),
    }
}

/// Composite an RGBA image onto a white background, dropping alpha.
pub fn flatten_alpha_to_rgb(img: &RgbaImage) -> image::RgbImage {
    let (w, h) = img.dimensions();
    let mut out = image::RgbImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y).0;
            let a = p[3] as f32 / 255.0;
            let bg = 255.0 * (1.0 - a);
            out.put_pixel(
                x,
                y,
                image::Rgb([
                    (p[0] as f32 * a + bg).round() as u8,
                    (p[1] as f32 * a + bg).round() as u8,
                    (p[2] as f32 * a + bg).round() as u8,
                ]),
            );
        }
    }
    out
}

/// Build an output path from an input path, an operation suffix, and an extension.
///
/// e.g. `photo.png + "cropped" + None` → `photo-cropped.png`
/// e.g. `photo.png + "cropped" + Some("jpg")` → `photo-cropped.jpg`
pub fn derive_output(input: &Path, suffix: &str, new_ext: Option<&str>) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());
    let ext = match new_ext {
        Some(e) => e.to_string(),
        None => input
            .extension()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "png".to_string()),
    };
    let mut p = PathBuf::from(input.parent().unwrap_or_else(|| Path::new(".")));
    p.push(format!("{stem}-{suffix}.{ext}"));
    p
}

/// Resolve an output path given:
/// - the global `--output` flag
/// - the number of input files
/// - the per-input derived path
///
/// Returns (path, is_dir_mode).
pub fn resolve_output(
    user_output: Option<&Path>,
    n_inputs: usize,
    derived: &Path,
) -> Result<PathBuf, Error> {
    match user_output {
        None => Ok(derived.to_path_buf()),
        Some(p) => {
            if n_inputs > 1 {
                // -o must be a directory for batch
                if p.exists() && !p.is_dir() {
                    return Err(Error::Usage(format!(
                        "--output must be a directory when processing multiple files: {}",
                        p.display()
                    )));
                }
                std::fs::create_dir_all(p)
                    .map_err(|e| Error::Processing(format!("could not create {}: {e}", p.display())))?;
                let filename = derived
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "output.png".to_string());
                Ok(p.join(filename))
            } else {
                // single file: -o is a path (or a directory we drop into)
                if p.is_dir() {
                    let filename = derived
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "output.png".to_string());
                    Ok(p.join(filename))
                } else {
                    Ok(p.to_path_buf())
                }
            }
        }
    }
}

/// Parse an aspect ratio string like "4:5" or "1.5" → (w, h).
pub fn parse_ratio(input: &str) -> Result<(f64, f64), Error> {
    if let Some((w, h)) = input.split_once(':') {
        let w: f64 = w
            .trim()
            .parse()
            .map_err(|_| Error::Usage(format!("invalid ratio: {input}")))?;
        let h: f64 = h
            .trim()
            .parse()
            .map_err(|_| Error::Usage(format!("invalid ratio: {input}")))?;
        if w <= 0.0 || h <= 0.0 {
            return Err(Error::Usage(format!("ratio components must be > 0: {input}")));
        }
        Ok((w, h))
    } else {
        let n: f64 = input
            .trim()
            .parse()
            .map_err(|_| Error::Usage(format!("invalid ratio: {input}")))?;
        if n <= 0.0 {
            return Err(Error::Usage(format!("ratio must be > 0: {input}")));
        }
        Ok((n, 1.0))
    }
}

/// Run a per-image batch operation.
///
/// `process(input_path)` should do the work and return one or more output paths
/// (and an optional `Value` of extra fields to merge into the result object).
///
/// Returns `Error::Processing` if any single file failed; never aborts the batch.
pub fn run_batch<F>(
    images: &[PathBuf],
    json: bool,
    quiet: bool,
    process: F,
) -> Result<(), Error>
where
    F: Fn(&Path) -> Result<BatchOk, Error>,
{
    if images.is_empty() {
        return Err(Error::Usage("no input files".into()));
    }
    let mut results: Vec<Value> = Vec::with_capacity(images.len());
    let mut failed = 0usize;

    for input in images {
        match process(input) {
            Ok(ok) => {
                if !quiet && !json {
                    for o in &ok.outputs {
                        println!("{} -> {}", input.display(), o.display());
                    }
                }
                let outs: Vec<String> =
                    ok.outputs.iter().map(|p| p.display().to_string()).collect();
                let mut obj = serde_json::Map::new();
                obj.insert("input".to_string(), json!(input.display().to_string()));
                if outs.len() == 1 {
                    obj.insert("output".to_string(), json!(outs[0]));
                } else {
                    obj.insert("outputs".to_string(), json!(outs));
                }
                obj.insert("ok".to_string(), json!(true));
                if let Some(Value::Object(extras)) = ok.extras {
                    for (k, v) in extras {
                        obj.insert(k, v);
                    }
                }
                results.push(Value::Object(obj));
            }
            Err(e) => {
                failed += 1;
                eprintln!("{}: {}", input.display(), e);
                let mut obj = serde_json::Map::new();
                obj.insert("input".to_string(), json!(input.display().to_string()));
                obj.insert("ok".to_string(), json!(false));
                obj.insert("error".to_string(), json!(e.to_string()));
                results.push(Value::Object(obj));
            }
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&Value::Array(results)).unwrap());
    }

    if failed > 0 {
        Err(Error::Processing(format!("{failed} of {} file(s) failed", images.len())))
    } else {
        Ok(())
    }
}

/// Result of a single file in a batch.
pub struct BatchOk {
    pub outputs: Vec<PathBuf>,
    pub extras: Option<Value>,
}

impl BatchOk {
    pub fn one(path: PathBuf) -> Self {
        Self { outputs: vec![path], extras: None }
    }
    pub fn with_extras(mut self, extras: Value) -> Self {
        self.extras = Some(extras);
        self
    }
}

/// Translate a user-supplied format string into the lower-case canonical extension
/// (e.g. "JPG" → "jpg", "jpeg" → "jpg").
pub fn canonical_ext(fmt: &str) -> &str {
    match fmt.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "jpg",
        "png" => "png",
        "webp" => "webp",
        "gif" => "gif",
        "tiff" | "tif" => "tif",
        "bmp" => "bmp",
        "ico" => "ico",
        _ => "png",
    }
}
