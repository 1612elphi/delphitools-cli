use crate::error::Error;
use crate::image_tools::{derive_output, open_image, parse_ratio, resolve_output, run_batch, BatchOk};
use std::path::{Path, PathBuf};

pub fn run(
    images: &[PathBuf],
    ratio: &str,
    position: &str,
    json: bool,
    quiet: bool,
    output: Option<&Path>,
) -> Result<(), Error> {
    let (rw, rh) = parse_ratio(ratio)?;
    let pos = parse_position(position)?;
    let n = images.len();

    run_batch(images, json, quiet, |input| {
        let img = open_image(input)?;

        let (iw, ih) = (img.width() as f64, img.height() as f64);
        let target_aspect = rw / rh;
        let img_aspect = iw / ih;

        let (cw, ch) = if target_aspect > img_aspect {
            (iw, iw / target_aspect)
        } else {
            (ih * target_aspect, ih)
        };
        let cw = cw.round().max(1.0) as u32;
        let ch = ch.round().max(1.0) as u32;
        let cw = cw.min(img.width());
        let ch = ch.min(img.height());

        let (x, y) = position_offset(pos, img.width(), img.height(), cw, ch);

        let mut img = img;
        let cropped = img.crop(x, y, cw, ch);

        let derived = derive_output(input, "cropped", None);
        let out_path = resolve_output(output, n, &derived)?;
        cropped
            .save(&out_path)
            .map_err(|e| Error::Processing(format!("could not save {}: {e}", out_path.display())))?;
        Ok(BatchOk::one(out_path).with_extras(serde_json::json!({
            "width": cw,
            "height": ch,
        })))
    })
}

#[derive(Clone, Copy)]
enum Position {
    Center,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

fn parse_position(s: &str) -> Result<Position, Error> {
    match s.to_ascii_lowercase().as_str() {
        "center" | "centre" | "middle" | "c" => Ok(Position::Center),
        "top" | "t" => Ok(Position::Top),
        "bottom" | "b" => Ok(Position::Bottom),
        "left" | "l" => Ok(Position::Left),
        "right" | "r" => Ok(Position::Right),
        "top-left" | "tl" | "topleft" => Ok(Position::TopLeft),
        "top-right" | "tr" | "topright" => Ok(Position::TopRight),
        "bottom-left" | "bl" | "bottomleft" => Ok(Position::BottomLeft),
        "bottom-right" | "br" | "bottomright" => Ok(Position::BottomRight),
        other => Err(Error::Usage(format!("invalid position: {other}"))),
    }
}

fn position_offset(p: Position, iw: u32, ih: u32, cw: u32, ch: u32) -> (u32, u32) {
    let dx = iw.saturating_sub(cw);
    let dy = ih.saturating_sub(ch);
    match p {
        Position::Center => (dx / 2, dy / 2),
        Position::Top => (dx / 2, 0),
        Position::Bottom => (dx / 2, dy),
        Position::Left => (0, dy / 2),
        Position::Right => (dx, dy / 2),
        Position::TopLeft => (0, 0),
        Position::TopRight => (dx, 0),
        Position::BottomLeft => (0, dy),
        Position::BottomRight => (dx, dy),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;
    use std::fs;

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("delphi-crop-test-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_image(path: &Path, w: u32, h: u32) {
        let mut img = RgbaImage::new(w, h);
        for p in img.pixels_mut() {
            p.0 = [255, 128, 0, 255];
        }
        img.save(path).unwrap();
    }

    #[test]
    fn crop_square_from_wide() {
        let dir = tmpdir("a");
        let input = dir.join("wide.png");
        make_image(&input, 200, 100);
        run(&[input.clone()], "1:1", "center", false, true, None).unwrap();
        let out = dir.join("wide-cropped.png");
        let img = image::open(&out).unwrap();
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn crop_position_top_left() {
        let dir = tmpdir("b");
        let input = dir.join("wide.png");
        make_image(&input, 200, 100);
        let out = dir.join("explicit.png");
        run(&[input], "1:1", "top-left", false, true, Some(&out)).unwrap();
        let img = image::open(&out).unwrap();
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn invalid_position_errors() {
        assert!(parse_position("nowhere").is_err());
    }

    #[test]
    fn ratio_parses() {
        assert_eq!(parse_ratio("16:9").unwrap(), (16.0, 9.0));
        assert_eq!(parse_ratio("4").unwrap(), (4.0, 1.0));
        assert!(parse_ratio("0:1").is_err());
    }
}
