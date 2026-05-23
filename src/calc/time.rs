use crate::error::Error;
use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde_json::{json, Map, Value};

// ---------------------------------------------------------------------------
// Input parsing
// ---------------------------------------------------------------------------

/// Parse the user's `input` argument into a UTC `DateTime`.
///
/// Recognised forms:
///   * `now`                → current UTC time
///   * pure integer         → unix timestamp (sec, or ms if > 1e11)
///   * RFC 3339 / ISO 8601  → e.g. `2026-04-09T12:00:00Z`, `2026-04-09T12:00:00+02:00`
///   * `YYYY-MM-DD HH:MM:SS`
///   * `YYYY-MM-DD`         → midnight UTC on that date
fn parse_input(input: &str) -> Result<DateTime<Utc>, Error> {
    let s = input.trim();

    if s.eq_ignore_ascii_case("now") {
        return Ok(Utc::now());
    }

    // Pure integer?  Optional leading sign, then all digits.
    let is_int = {
        let mut bytes = s.as_bytes().iter();
        if let Some(&b) = bytes.next() {
            let rest_all_digit = bytes.all(|c| c.is_ascii_digit());
            (b == b'-' || b == b'+' || b.is_ascii_digit()) && rest_all_digit && s.len() > 1
                || (b.is_ascii_digit() && s.len() == 1)
        } else {
            false
        }
    };
    if is_int {
        let n: i64 = s
            .parse()
            .map_err(|_| Error::Input(format!("invalid integer timestamp '{s}'")))?;
        let dt = if n.unsigned_abs() > 100_000_000_000 {
            // Treat as milliseconds.
            DateTime::<Utc>::from_timestamp_millis(n)
                .ok_or_else(|| Error::Input(format!("timestamp out of range: {n}")))?
        } else {
            DateTime::<Utc>::from_timestamp(n, 0)
                .ok_or_else(|| Error::Input(format!("timestamp out of range: {n}")))?
        };
        return Ok(dt);
    }

    // RFC 3339 / ISO 8601 with timezone.
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // RFC 2822.
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Ok(dt.with_timezone(&Utc));
    }

    // ISO 8601 variant without timezone — treat as UTC.
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&dt));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(Utc.from_utc_datetime(&dt));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&dt));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        return Ok(Utc.from_utc_datetime(&dt));
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| Error::Input(format!("invalid date '{s}'")))?;
        return Ok(Utc.from_utc_datetime(&dt));
    }

    Err(Error::Input(format!(
        "could not parse '{s}' as a date or timestamp"
    )))
}

// ---------------------------------------------------------------------------
// Duration parsing
// ---------------------------------------------------------------------------

/// Parse a duration string like `30d`, `5h`, `90m`, `15s`, `2w`, `3mo`, `1y`.
///
/// Returns chrono `Duration`. Approximates month = 30d, year = 365d.
fn parse_duration(s: &str) -> Result<Duration, Error> {
    let s = s.trim();
    if s.is_empty() {
        return Err(Error::Usage("empty duration".into()));
    }
    // Split off the numeric prefix.
    let bytes = s.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        i += 1;
    }
    if i == 0 || (i == 1 && !bytes[0].is_ascii_digit()) {
        return Err(Error::Usage(format!("invalid duration '{s}'")));
    }
    let num_str = &s[..i];
    let unit = s[i..].trim().to_lowercase();
    let value: f64 = num_str
        .parse()
        .map_err(|_| Error::Usage(format!("invalid number in duration '{s}'")))?;

    // Seconds-per-unit (float for fractional values).
    let secs_per = match unit.as_str() {
        "ms" => 0.001,
        "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
        "m" | "min" | "mins" | "minute" | "minutes" => 60.0,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3600.0,
        "d" | "day" | "days" => 86400.0,
        "w" | "wk" | "wks" | "week" | "weeks" => 604800.0,
        "mo" | "mos" | "month" | "months" => 86400.0 * 30.0,
        "y" | "yr" | "yrs" | "year" | "years" => 86400.0 * 365.0,
        other => return Err(Error::Usage(format!("unknown duration unit '{other}'"))),
    };
    let total_secs = value * secs_per;
    // Use milliseconds resolution so fractional values survive.
    let ms = (total_secs * 1000.0).round() as i64;
    Ok(Duration::milliseconds(ms))
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum Format {
    Iso,
    Unix,
    Rfc2822,
    Rfc3339,
    Human,
}

impl Format {
    fn key(self) -> &'static str {
        match self {
            Format::Iso => "iso",
            Format::Unix => "unix",
            Format::Rfc2822 => "rfc2822",
            Format::Rfc3339 => "rfc3339",
            Format::Human => "human",
        }
    }
}

fn parse_formats(spec: Option<&str>) -> Result<Vec<Format>, Error> {
    match spec {
        None => Ok(vec![
            Format::Iso,
            Format::Rfc2822,
            Format::Rfc3339,
            Format::Human,
        ]),
        Some(s) => {
            let mut out = Vec::new();
            for tok in s.split(',') {
                let tok = tok.trim().to_lowercase();
                let f = match tok.as_str() {
                    "iso" | "iso8601" => Format::Iso,
                    "unix" | "epoch" => Format::Unix,
                    "rfc2822" => Format::Rfc2822,
                    "rfc3339" => Format::Rfc3339,
                    "human" => Format::Human,
                    other => {
                        return Err(Error::Usage(format!(
                            "unknown format '{other}' (expected iso, unix, rfc2822, rfc3339, human)"
                        )))
                    }
                };
                out.push(f);
            }
            if out.is_empty() {
                return Err(Error::Usage("empty --to list".into()));
            }
            Ok(out)
        }
    }
}

/// Render a UTC datetime (already shifted to the requested zone) in a given format.
/// For unix output the *original* UTC datetime is used because epoch seconds are
/// zone-agnostic.
fn render(zoned: &DateTime<Tz>, utc: DateTime<Utc>, fmt: Format) -> String {
    match fmt {
        Format::Iso => zoned.format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
        Format::Rfc2822 => zoned.to_rfc2822(),
        Format::Rfc3339 => zoned.to_rfc3339(),
        Format::Human => zoned.format("%a, %d %b %Y %H:%M:%S %Z").to_string(),
        Format::Unix => utc.timestamp().to_string(),
    }
}

// ---------------------------------------------------------------------------
// run
// ---------------------------------------------------------------------------

pub fn run(
    input: Option<&str>,
    to: Option<&str>,
    tz: Option<&str>,
    add: Option<&str>,
    sub: Option<&str>,
    as_json: bool,
) -> Result<(), Error> {
    // Resolve the input datetime.
    let mut dt: DateTime<Utc> = match input {
        Some(s) if !s.is_empty() => parse_input(s)?,
        _ => Utc::now(),
    };

    if let Some(d) = add {
        let dur = parse_duration(d)?;
        dt = dt
            .checked_add_signed(dur)
            .ok_or_else(|| Error::Processing("overflow when adding duration".into()))?;
    }
    if let Some(d) = sub {
        let dur = parse_duration(d)?;
        dt = dt
            .checked_sub_signed(dur)
            .ok_or_else(|| Error::Processing("overflow when subtracting duration".into()))?;
    }

    // Resolve timezone (default UTC).
    let zone: Tz = match tz {
        Some(name) => name
            .parse()
            .map_err(|_| Error::Usage(format!("unknown timezone '{name}'")))?,
        None => chrono_tz::UTC,
    };
    let zoned = dt.with_timezone(&zone);

    let formats = parse_formats(to)?;

    if as_json {
        let mut obj = Map::new();
        let mut included_unix = false;
        for f in &formats {
            let v = match f {
                Format::Unix => {
                    included_unix = true;
                    json!(dt.timestamp())
                }
                _ => json!(render(&zoned, dt, *f)),
            };
            obj.insert(f.key().to_string(), v);
        }
        // Always include unix in JSON output for convenience.
        if !included_unix {
            obj.insert("unix".to_string(), json!(dt.timestamp()));
        }
        println!("{}", serde_json::to_string_pretty(&Value::Object(obj)).unwrap());
        return Ok(());
    }

    // Plain output. Single format → bare line. Otherwise, label : value lines.
    if formats.len() == 1 {
        println!("{}", render(&zoned, dt, formats[0]));
        return Ok(());
    }

    let labels: Vec<&str> = formats.iter().map(|f| f.key()).collect();
    let mut have_unix = labels.contains(&"unix");
    let mut width = labels.iter().map(|s| s.len()).max().unwrap_or(4);
    if !have_unix {
        width = width.max(4); // "unix"
    }
    for f in &formats {
        println!("{:<width$}: {}", f.key(), render(&zoned, dt, *f), width = width);
        if matches!(f, Format::Unix) {
            have_unix = true;
        }
    }
    if !have_unix {
        println!("{:<width$}: {}", "unix", dt.timestamp(), width = width);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_parse_now() {
        let _ = parse_input("now").unwrap();
        let _ = parse_input("NOW").unwrap();
    }

    #[test]
    fn time_parse_unix_seconds() {
        let dt = parse_input("1744209000").unwrap();
        assert_eq!(dt.timestamp(), 1744209000);
    }

    #[test]
    fn time_parse_unix_milliseconds() {
        // 1744209000000 ms = 1744209000 s
        let dt = parse_input("1744209000000").unwrap();
        assert_eq!(dt.timestamp(), 1744209000);
    }

    #[test]
    fn time_parse_date_only() {
        let dt = parse_input("2026-04-09").unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2026-04-09 00:00:00");
    }

    #[test]
    fn time_parse_iso_no_tz() {
        let dt = parse_input("2026-04-09T12:30:45").unwrap();
        assert_eq!(dt.timestamp(), 1775737845);
    }

    #[test]
    fn time_parse_iso_with_tz() {
        let dt = parse_input("2026-04-09T12:30:45+02:00").unwrap();
        // 12:30:45 +02 == 10:30:45 UTC
        assert_eq!(dt.format("%H:%M:%S").to_string(), "10:30:45");
    }

    #[test]
    fn time_parse_human() {
        let dt = parse_input("2026-04-09 12:30:45").unwrap();
        assert_eq!(dt.timestamp(), 1775737845);
    }

    #[test]
    fn time_parse_z_suffix() {
        let dt = parse_input("2026-04-09T12:30:45Z").unwrap();
        assert_eq!(dt.timestamp(), 1775737845);
    }

    #[test]
    fn time_parse_bad_input_errors() {
        assert!(parse_input("not a date").is_err());
    }

    #[test]
    fn time_duration_seconds() {
        assert_eq!(parse_duration("15s").unwrap().num_seconds(), 15);
    }

    #[test]
    fn time_duration_minutes() {
        assert_eq!(parse_duration("90m").unwrap().num_seconds(), 5400);
    }

    #[test]
    fn time_duration_hours() {
        assert_eq!(parse_duration("5h").unwrap().num_seconds(), 5 * 3600);
    }

    #[test]
    fn time_duration_days() {
        assert_eq!(parse_duration("30d").unwrap().num_seconds(), 30 * 86400);
    }

    #[test]
    fn time_duration_weeks() {
        assert_eq!(parse_duration("2w").unwrap().num_seconds(), 2 * 7 * 86400);
    }

    #[test]
    fn time_duration_months() {
        assert_eq!(parse_duration("3mo").unwrap().num_seconds(), 3 * 30 * 86400);
    }

    #[test]
    fn time_duration_years() {
        assert_eq!(parse_duration("1y").unwrap().num_seconds(), 365 * 86400);
    }

    #[test]
    fn time_duration_ms() {
        assert_eq!(parse_duration("250ms").unwrap().num_milliseconds(), 250);
    }

    #[test]
    fn time_duration_unknown_unit_errors() {
        assert!(parse_duration("5x").is_err());
    }

    #[test]
    fn time_run_now_smoke() {
        run(None, None, None, None, None, false).unwrap();
        run(Some("now"), None, None, None, None, false).unwrap();
    }

    #[test]
    fn time_run_unix_input() {
        run(
            Some("1744209000"),
            Some("iso"),
            None,
            None,
            None,
            false,
        )
        .unwrap();
    }

    #[test]
    fn time_run_with_add() {
        run(
            Some("2026-04-09"),
            Some("iso"),
            None,
            Some("30d"),
            None,
            false,
        )
        .unwrap();
    }

    #[test]
    fn time_run_with_tz() {
        run(
            Some("2026-04-09T12:00:00Z"),
            Some("iso"),
            Some("America/New_York"),
            None,
            None,
            false,
        )
        .unwrap();
    }

    #[test]
    fn time_run_bad_tz() {
        let r = run(
            Some("now"),
            None,
            Some("Mars/Phobos"),
            None,
            None,
            false,
        );
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn time_run_bad_format() {
        let r = run(Some("now"), Some("klingon"), None, None, None, false);
        assert!(matches!(r, Err(Error::Usage(_))));
    }

    #[test]
    fn time_run_json() {
        run(Some("1744209000"), None, None, None, None, true).unwrap();
    }
}
