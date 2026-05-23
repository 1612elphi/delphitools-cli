mod calc;
mod cli;
mod colour;
mod error;
mod gen;
mod image_tools;
mod input;
mod output;
mod pdf;
mod text;

use clap::{CommandFactory, Parser};
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(e.exit_code());
    }
}

fn run(cli: Cli) -> Result<(), error::Error> {
    let out = cli.output;
    let json = cli.json;
    let quiet = cli.quiet;

    match cli.command {
        // ── Colour ──────────────────────────────────────────────────────────
        Command::Colour { colour, formats, pretty } => {
            colour::convert::run(&colour, &formats, json, pretty)
        }
        Command::Harmony { colour, harmony_type, pretty } => {
            colour::harmony::run(&colour, harmony_type.as_deref(), json, pretty)
        }
        Command::Contrast { fg, bg } => colour::contrast::run(&fg, &bg, json),
        Command::TailwindShades { colour, mode, pretty } => {
            colour::shades::run(&colour, mode.as_deref(), json, pretty)
        }
        Command::Palette { strategy, size, format, lock, seed, pretty, list } => {
            colour::palette::run(
                strategy.as_deref(),
                size,
                &format,
                lock.as_deref(),
                seed,
                pretty,
                list,
                json,
                out.as_deref(),
            )
        }
        Command::Colorblind { input, cb_type, colour } => {
            colour::colorblind::run(
                input.as_deref(),
                &cb_type,
                colour,
                json,
                out.as_deref(),
            )
        }

        // ── Social Media ────────────────────────────────────────────────────
        Command::Crop { images, ratio, position } => {
            image_tools::crop::run(&images, &ratio, &position, json, quiet, out.as_deref())
        }
        Command::Matte { images, ratio, mode, colour } => {
            image_tools::matte::run(&images, &ratio, &mode, &colour, json, quiet, out.as_deref())
        }
        Command::Scroll { image, ratio, fill, colour } => {
            image_tools::scroll::run(&image, &ratio, &fill, &colour, json, quiet, out.as_deref())
        }
        Command::Watermark { images, mark, position, opacity, scale } => {
            image_tools::watermark::run(
                &images, &mark, &position, opacity, scale, json, quiet, out.as_deref(),
            )
        }

        // ── Images & Assets ─────────────────────────────────────────────────
        Command::Favicon { image, sizes, ico } => {
            image_tools::favicon::run(&image, &sizes, ico, json, quiet, out.as_deref())
        }
        Command::Svgo { files } => image_tools::svgo::run(&files, json, quiet, out.as_deref()),
        Command::Split { image, rows, cols } => {
            image_tools::split::run(&image, rows, cols, json, quiet, out.as_deref())
        }
        Command::Convert { images, to, quality, resize } => {
            image_tools::convert::run(
                &images,
                &to,
                quality,
                resize.as_deref(),
                json,
                quiet,
                out.as_deref(),
            )
        }
        Command::Noise { images, opacity, scale, seed } => {
            image_tools::noise::run(
                &images,
                opacity,
                scale,
                seed,
                json,
                quiet,
                out.as_deref(),
            )
        }
        Command::Rmbg { images } => {
            image_tools::rmbg::run(&images, json, quiet, out.as_deref())
        }
        Command::Trace { image, preset, colours, blur } => {
            image_tools::trace::run(
                &image,
                &preset,
                colours,
                blur,
                json,
                quiet,
                out.as_deref(),
            )
        }
        Command::Clip { images } => {
            image_tools::clip::run(&images, json, quiet, out.as_deref())
        }

        // ── Typography ──────────────────────────────────────────────────────
        Command::Px2rem { value, base } => text::px2rem::run_px2rem(value, base, json),
        Command::Rem2px { value, base } => text::px2rem::run_rem2px(value, base, json),
        Command::LineHeight { font_size, name } => {
            text::line_height::run(font_size, name.as_deref(), json)
        }
        Command::Typo { value, targets, base } => text::typo::run(&value, &targets, base, json),
        Command::Wc { input } => text::wc::run(input.as_deref(), json),
        Command::Paper { name, series, unit, dpi, pixels } => {
            text::paper::run(name.as_deref(), series.as_deref(), &unit, dpi, pixels, json)
        }
        Command::Glyph { input, range, search, limit } => {
            text::glyph::run(
                input.as_deref(),
                range.as_deref(),
                search.as_deref(),
                limit,
                json,
            )
        }
        Command::FontInfo { font } => text::font_info::run(&font, json),
        Command::Shavian { input, gloss } => text::shavian::run(input.as_deref(), gloss, json),

        // ── Print & Production ──────────────────────────────────────────────
        Command::Preflight { pdf } => pdf::preflight::run(&pdf, json),
        Command::Zine { images, paper, dpi } => {
            pdf::zine::run(&images, &paper, dpi, json, quiet, out.as_deref())
        }
        Command::Impose {
            pdf,
            layout,
            paper,
            n_up,
            signature,
            margins,
            gutter,
            creep,
            crop_marks,
            duplex,
        } => pdf::impose::run(
            &pdf, &layout, &paper, n_up, signature, margins, gutter, creep, crop_marks, duplex,
            json, quiet, out.as_deref(),
        ),

        // ── Generators ──────────────────────────────────────────────────────
        Command::Qr {
            data,
            size,
            fg,
            bg,
            logo,
            error_level,
        } => gen::qr::run(
            &data,
            size,
            &fg,
            &bg,
            logo.as_deref(),
            &error_level,
            json,
            quiet,
            out.as_deref(),
        ),
        Command::Barcode {
            data,
            format,
            height,
            scale,
            text,
        } => gen::barcode::run(
            &data,
            &format,
            height,
            scale,
            text,
            json,
            quiet,
            out.as_deref(),
        ),
        Command::Meta {
            title,
            description,
            url,
            image,
            page_type,
            site_name,
            author,
            twitter_handle,
        } => gen::meta::run(
            &title,
            &description,
            url.as_deref(),
            image.as_deref(),
            &page_type,
            site_name.as_deref(),
            author.as_deref(),
            twitter_handle.as_deref(),
            json,
        ),
        Command::Regex {
            pattern,
            text,
            flags,
        } => text::regex_tool::run(&pattern, text.as_deref(), &flags, json),

        // ── Calculators ─────────────────────────────────────────────────────
        Command::Calc { expression, angles } => {
            calc::calc::run(expression.as_deref(), &angles, json)
        }
        Command::Base { value, targets, from } => calc::base::run(&value, &targets, &from, json),
        Command::Time {
            input,
            to,
            tz,
            add,
            sub,
        } => calc::time::run(
            input.as_deref(),
            to.as_deref(),
            tz.as_deref(),
            add.as_deref(),
            sub.as_deref(),
            json,
        ),
        Command::Unit { value, targets } => calc::unit::run(&value, &targets, json),

        Command::Encode { encoding, input } => calc::encode::run_encode(&encoding, input.as_deref()),
        Command::Decode { encoding, input } => calc::encode::run_decode(&encoding, input.as_deref()),
        Command::Hash { algorithm, input } => calc::encode::run_hash(&algorithm, input.as_deref()),

        // ── meta ────────────────────────────────────────────────────────────
        Command::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "delphi", &mut std::io::stdout());
            Ok(())
        }
    }
}
