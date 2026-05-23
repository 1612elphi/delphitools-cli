//! Background removal via ISNet-General-Use (Apache 2.0).
//!
//! Pulls a ~170 MB ONNX model from the `rembg` project's GitHub releases on
//! first use, caches it under `$HOME/.cache/delphitools-cli/models/`, and
//! runs CPU inference via the `ort` crate (ONNX Runtime).
//!
//! Network use is gated:
//! * In a TTY, the user gets a yes/no prompt before any download.
//! * Otherwise, `--approve` is required.
//! After the model lands in the cache, neither prompt nor flag is needed again.

use crate::error::Error;
use crate::image_tools::{derive_output, open_image, resolve_output};
use image::{ImageBuffer, Rgba, RgbaImage};
use ort::session::Session;
use ort::value::Tensor;
use std::io::{IsTerminal, Read, Write};
use std::process::Command;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Model constants
// ---------------------------------------------------------------------------

const MODEL_FILENAME: &str = "isnet-general-use.onnx";
const MODEL_URL: &str =
    "https://github.com/danielgatis/rembg/releases/download/v0.0.0/isnet-general-use.onnx";
const MODEL_SIZE_HINT: &str = "~170 MB";
const MODEL_LICENSE: &str = "Apache 2.0";
// Expected on-disk size (from rembg releases). Verified at runtime so we
// catch truncated downloads before sending them to ONNX Runtime.
const MODEL_EXPECTED_BYTES: u64 = 178_648_008;

// ISNet input dimensions (square; the model resizes any input to this).
const INPUT_SIZE: u32 = 1024;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run(
    images: &[PathBuf],
    approve: bool,
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    if images.is_empty() {
        return Err(Error::Usage("rmbg: no input images".into()));
    }

    let model_path = ensure_model_cached(approve, quiet)?;

    // Load the ONNX session once and reuse it across every input image.
    let mut session = Session::builder()
        .map_err(|e| Error::Processing(format!("rmbg: ort init: {e}")))?
        .commit_from_file(&model_path)
        .map_err(|e| Error::Processing(format!("rmbg: load model: {e}")))?;

    let n = images.len();
    let mut results: Vec<serde_json::Value> = Vec::with_capacity(n);
    let mut failed = 0usize;

    for input in images {
        match remove_one(&mut session, input, n, output) {
            Ok(out_path) => {
                if !quiet && !json {
                    println!("{} -> {}", input.display(), out_path.display());
                }
                results.push(serde_json::json!({
                    "input": input.display().to_string(),
                    "output": out_path.display().to_string(),
                    "ok": true,
                }));
            }
            Err(e) => {
                failed += 1;
                eprintln!("{}: {}", input.display(), e);
                results.push(serde_json::json!({
                    "input": input.display().to_string(),
                    "ok": false,
                    "error": e.to_string(),
                }));
            }
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::Value::Array(results)).unwrap()
        );
    }

    if failed > 0 {
        return Err(Error::Processing(format!(
            "{failed} of {n} file(s) failed"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Per-image pipeline
// ---------------------------------------------------------------------------

fn remove_one(
    session: &mut Session,
    input: &Path,
    n_inputs: usize,
    output: Option<&Path>,
) -> Result<PathBuf, Error> {
    let img = open_image(input)?.to_rgba8();
    let (orig_w, orig_h) = (img.width(), img.height());

    // Preprocess: resize → CHW float32, normalised to (x/255 − 0.5).
    let resized = image::imageops::resize(
        &img,
        INPUT_SIZE,
        INPUT_SIZE,
        image::imageops::FilterType::Triangle,
    );
    let chw = preprocess_chw(&resized);
    let n = INPUT_SIZE as usize;
    let tensor = Tensor::from_array(([1_usize, 3, n, n], chw))
        .map_err(|e| Error::Processing(format!("rmbg: build input tensor: {e}")))?;

    // Inference.
    let outputs = session
        .run(ort::inputs![tensor])
        .map_err(|e| Error::Processing(format!("rmbg: inference: {e}")))?;

    // ISNet returns multiple side outputs; the first is the highest-resolution mask.
    let (_shape, mask_data) = outputs[0]
        .try_extract_tensor::<f32>()
        .map_err(|e| Error::Processing(format!("rmbg: extract: {e}")))?;

    let mask = postprocess(mask_data, INPUT_SIZE, INPUT_SIZE);

    // Resize mask back to original dimensions.
    let mask_resized = image::imageops::resize(
        &mask,
        orig_w,
        orig_h,
        image::imageops::FilterType::Triangle,
    );

    // Apply mask as the alpha channel of the original.
    let mut out = RgbaImage::new(orig_w, orig_h);
    for y in 0..orig_h {
        for x in 0..orig_w {
            let p = img.get_pixel(x, y).0;
            let a = mask_resized.get_pixel(x, y).0[0];
            out.put_pixel(x, y, Rgba([p[0], p[1], p[2], a]));
        }
    }

    let derived = derive_output(input, "nobg", Some("png"));
    let out_path = resolve_output(output, n_inputs, &derived)?;
    out.save(&out_path)
        .map_err(|e| Error::Processing(format!("could not save {}: {e}", out_path.display())))?;
    Ok(out_path)
}

/// Convert an RGBA image to a CHW-laid-out flat `Vec<f32>` of length
/// `3 * H * W`, normalised by `(x / 255) - 0.5`. Alpha is dropped.
fn preprocess_chw(img: &RgbaImage) -> Vec<f32> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut out = vec![0.0f32; 3 * h * w];
    let plane = h * w;
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x as u32, y as u32).0;
            let idx = y * w + x;
            out[idx] = (p[0] as f32 / 255.0) - 0.5;
            out[plane + idx] = (p[1] as f32 / 255.0) - 0.5;
            out[2 * plane + idx] = (p[2] as f32 / 255.0) - 0.5;
        }
    }
    out
}

/// Normalise the model's mask output to a single-channel 8-bit image. ISNet's
/// output is already roughly in [0, 1] but can drift slightly; rescale to its
/// own min/max before quantising so we use the full alpha range.
fn postprocess(
    data: &[f32],
    w: u32,
    h: u32,
) -> ImageBuffer<image::Luma<u8>, Vec<u8>> {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for &v in data {
        if v < lo {
            lo = v;
        }
        if v > hi {
            hi = v;
        }
    }
    let span = (hi - lo).max(f32::EPSILON);
    let mut buf = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let v = data[(y * w + x) as usize];
            let norm = ((v - lo) / span).clamp(0.0, 1.0);
            buf.put_pixel(x, y, image::Luma([(norm * 255.0).round() as u8]));
        }
    }
    buf
}

// ---------------------------------------------------------------------------
// Model cache + download
// ---------------------------------------------------------------------------

fn cache_dir() -> Result<PathBuf, Error> {
    let base = dirs::cache_dir().ok_or_else(|| {
        Error::Processing("rmbg: could not resolve cache dir (set HOME)".into())
    })?;
    Ok(base.join("delphitools-cli").join("models"))
}

fn ensure_model_cached(approve: bool, quiet: bool) -> Result<PathBuf, Error> {
    let dir = cache_dir()?;
    let path = dir.join(MODEL_FILENAME);

    if path.exists() {
        // Sanity check: if a previous download was interrupted, redo it.
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() >= MODEL_EXPECTED_BYTES - 1024 * 1024
                && meta.len() <= MODEL_EXPECTED_BYTES + 1024 * 1024
            {
                return Ok(path);
            }
            // Size mismatch — fall through and re-download.
            if !quiet {
                eprintln!(
                    "rmbg: cached model size mismatch ({} bytes), re-downloading",
                    meta.len()
                );
            }
        }
    }

    // Need to download. Get consent.
    if !approve && !confirm_download(&path)? {
        return Err(Error::Usage(
            "rmbg: model download not approved. Run with --approve to download, \
             or remove the rmbg invocation."
                .into(),
        ));
    }

    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::Processing(format!("rmbg: create cache dir: {e}")))?;

    download(MODEL_URL, &path, quiet)?;

    let actual = std::fs::metadata(&path)
        .map(|m| m.len())
        .unwrap_or(0);
    if actual < MODEL_EXPECTED_BYTES - 1024 * 1024 {
        // Cleanup partial file so the next run retries cleanly.
        let _ = std::fs::remove_file(&path);
        return Err(Error::Processing(format!(
            "rmbg: download truncated ({} of ~{} bytes)",
            actual, MODEL_EXPECTED_BYTES
        )));
    }
    Ok(path)
}

fn confirm_download(path: &Path) -> Result<bool, Error> {
    let stdin_tty = std::io::stdin().is_terminal();
    let stderr_tty = std::io::stderr().is_terminal();
    if !stdin_tty || !stderr_tty {
        // Non-interactive — refuse silently. Caller turns this into a usage error.
        return Ok(false);
    }

    eprintln!();
    eprintln!("┌─ rmbg ────────────────────────────────────────────────────────┐");
    eprintln!("│ This command needs a one-time {MODEL_SIZE_HINT} background-removal model.    │");
    eprintln!("│ License: {MODEL_LICENSE} (no commercial restrictions)              │");
    eprintln!("│ Source : github.com/danielgatis/rembg releases                │");
    eprintln!("│ Cache  : {}", abbreviate(path));
    eprintln!("└───────────────────────────────────────────────────────────────┘");
    eprint!("Download now? [y/N] ");
    std::io::stderr().flush().ok();

    let mut buf = [0u8; 8];
    let n = std::io::stdin()
        .read(&mut buf)
        .map_err(|e| Error::Processing(format!("rmbg: read stdin: {e}")))?;
    let answer = String::from_utf8_lossy(&buf[..n]).trim().to_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

fn abbreviate(p: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(rest) = p.strip_prefix(&home) {
            return format!("~/{}", rest.display());
        }
    }
    p.display().to_string()
}

fn download(url: &str, dest: &Path, quiet: bool) -> Result<(), Error> {
    if !quiet {
        eprintln!("rmbg: downloading {MODEL_FILENAME}…");
    }
    let tmp_path = dest.with_extension("onnx.partial");

    // Use curl for the actual fetch. It's universally available on macOS/Linux,
    // handles redirects/TLS robustly, supports resume on flaky connections, and
    // displays its own progress bar when stderr is a TTY. (We tried ureq here
    // and it stalled mid-download against GitHub's release CDN.)
    let show_progress = !quiet && std::io::stderr().is_terminal();
    let progress_flag = if show_progress {
        "--progress-bar"
    } else {
        "--silent"
    };

    let status = Command::new("curl")
        .arg("--fail")              // non-2xx → exit non-zero
        .arg("--location")          // follow 302
        .arg(progress_flag)
        .arg("--show-error")        // still print errors when --silent
        .arg("--continue-at")       // resume if .partial already exists
        .arg("-")
        .arg("--output")
        .arg(&tmp_path)
        .arg(url)
        .status()
        .map_err(|e| Error::Processing(format!("rmbg: curl spawn failed ({e}); is curl installed?")))?;

    if !status.success() {
        return Err(Error::Processing(format!(
            "rmbg: curl exited with status {} (download failed)",
            status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into())
        )));
    }

    std::fs::rename(&tmp_path, dest)
        .map_err(|e| Error::Processing(format!("rmbg: finalise: {e}")))?;
    Ok(())
}
