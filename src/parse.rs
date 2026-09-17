//! Hand-written parsers for the free-form inputs: time, date and offset.

use crate::error::{Error, Result};
use jiff::{
    civil,
    tz::{Offset, TimeZone},
};
use std::ops::RangeInclusive;

const MERIDIEM_HOURS: RangeInclusive<i8> = 1..=12;
const HALF_DAY: i8 = 12;
const SECS_PER_HOUR: i32 = 3600;
const SECS_PER_MIN: i32 = 60;

/// Parses `8.30 pm`, `8:30pm`, `8pm` or `20:30`.
///
/// # Errors
///
/// Returns [Error::Time] for anything that is not a clock time.
pub(crate) fn parse_time(input: &str) -> Result<civil::Time> {
    clock(input).ok_or_else(|| Error::Time(input.to_string()))
}

/// Parses `YYYY-MM-DD`.
///
/// # Errors
///
/// Returns [Error::Date] for anything that is not a calendar date.
pub(crate) fn parse_date(input: &str) -> Result<civil::Date> {
    input.parse().map_err(|_| Error::Date(input.to_string()))
}

/// Parses `GMT+4`, `UTC+5:30`, `+04:00` or `UTC` into a fixed-offset zone.
///
/// `None` means the input is not an offset, so the caller should try an IANA
/// name. Parsed by hand on purpose: IANA `Etc/GMT+4` means UTC-4 (POSIX sign
/// inversion), while a person typing `GMT+4` means UTC+4.
pub(crate) fn parse_offset(s: &str) -> Option<TimeZone> {
    let up = s.to_uppercase();
    let rest = up
        .strip_prefix("GMT")
        .or_else(|| up.strip_prefix("UTC"))
        .unwrap_or(&up);
    if rest.is_empty() {
        return (rest.len() != up.len()).then_some(TimeZone::UTC);
    }
    let sign = match rest.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (h, m) = rest[1..].split_once(':').unwrap_or((&rest[1..], "0"));
    let secs =
        sign * (h.parse::<i32>().ok()? * SECS_PER_HOUR + m.parse::<i32>().ok()? * SECS_PER_MIN);
    Some(TimeZone::fixed(Offset::from_seconds(secs).ok()?))
}

fn clock(input: &str) -> Option<civil::Time> {
    let s = input.to_lowercase().replace(' ', "");
    let (clock, pm) = match (s.strip_suffix("am"), s.strip_suffix("pm")) {
        (Some(c), _) => (c, Some(false)),
        (_, Some(c)) => (c, Some(true)),
        _ => (s.as_str(), None),
    };
    let (h, m) = clock.split_once([':', '.']).unwrap_or((clock, "0"));
    let (mut h, m): (i8, i8) = (h.parse().ok()?, m.parse().ok()?);
    if let Some(pm) = pm {
        if !MERIDIEM_HOURS.contains(&h) {
            return None;
        }
        h = h % HALF_DAY + if pm { HALF_DAY } else { 0 };
    }
    civil::Time::new(h, m, 0, 0).ok()
}

#[cfg(test)]
mod tests {
    use super::{parse_date, parse_offset, parse_time};
    use jiff::civil;

    #[test]
    fn times() {
        let t = |s| parse_time(s).unwrap();
        assert_eq!(t("8.30pm"), civil::time(20, 30, 0, 0));
        assert_eq!(t("8:30 PM"), civil::time(20, 30, 0, 0));
        assert_eq!(t("8pm"), civil::time(20, 0, 0, 0));
        assert_eq!(t("12am"), civil::time(0, 0, 0, 0));
        assert_eq!(t("12pm"), civil::time(12, 0, 0, 0));
        assert_eq!(t("20:30"), civil::time(20, 30, 0, 0));
        for bad in ["13pm", "0am", "25:00", "8:60", "soon"] {
            assert!(parse_time(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn dates() {
        assert_eq!(parse_date("2026-09-22").unwrap(), civil::date(2026, 9, 22));
        for bad in ["22/09/2026", "2026-13-01", "tomorrow"] {
            assert!(parse_date(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn offsets() {
        let secs = |s| {
            parse_offset(s)
                .unwrap()
                .to_fixed_offset()
                .unwrap()
                .seconds()
        };
        assert_eq!(secs("GMT+4"), 4 * 3600); // east of Greenwich, not POSIX-inverted
        assert_eq!(secs("utc+5:30"), 5 * 3600 + 1800);
        assert_eq!(secs("-04:00"), -4 * 3600);
        assert_eq!(secs("UTC"), 0);
        assert!(parse_offset("Asia/Kolkata").is_none());
        assert!(parse_offset("GMT+99").is_none());
    }
}
