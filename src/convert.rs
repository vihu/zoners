//! The conversion itself: one moment in a source zone, read in every place.

use crate::{
    band::Band,
    error::{Error, Result},
    parse::{parse_date, parse_offset, parse_time},
    places::Place,
};
use jiff::{tz::TimeZone, Zoned};

/// One place at the requested moment.
#[derive(Clone, Debug)]
pub struct Row {
    /// Name of the place.
    pub label: String,
    /// The moment, in this place's zone.
    pub at: Zoned,
    /// Part of the day the local hour falls in.
    pub band: Band,
    /// Whether this place keeps the source zone.
    pub is_source: bool,
    /// Days from the source date to this place's date: -1, 0 or 1.
    pub day_shift: i32,
}

/// The moment to convert, as typed by the user.
///
/// Every field is optional and falls back to now in the system zone.
#[derive(Clone, Copy, Debug, Default)]
pub struct Query<'a> {
    /// Clock time such as `20:30`, `8.30pm` or `8pm`.
    pub time: Option<&'a str>,
    /// Calendar date as `YYYY-MM-DD`.
    pub date: Option<&'a str>,
    /// Source zone: a place label, an offset such as `GMT+4`, or an IANA name.
    pub zone: Option<&'a str>,
}

const SYSTEM_LABEL: &str = "local";

/// Reads the queried moment in every place.
///
/// The source zone leads the rows unless one of `places` already keeps it.
///
/// ```
/// use zoners::{convert, parse_places, Query};
///
/// let places = parse_places("UTC Greenwich")?;
/// let query = Query { time: Some("8.30pm"), date: Some("2026-09-22"), zone: Some("GMT+4") };
/// let rows = convert(query, &places)?;
/// assert_eq!(rows[0].label, "GMT+4");
/// assert_eq!(rows[1].at.strftime("%H:%M").to_string(), "16:30");
/// # Ok::<(), zoners::Error>(())
/// ```
///
/// # Errors
///
/// Returns [Error::Time], [Error::Date] or [Error::Zone] for input that does
/// not parse, and [Error::Moment] if the date is out of range for the zone.
pub fn convert(query: Query<'_>, places: &[Place]) -> Result<Vec<Row>> {
    let (source, source_label) = match query.zone {
        None => (TimeZone::system(), SYSTEM_LABEL.to_string()),
        Some(input) => (source_zone(input, places)?, input.to_string()),
    };
    let now = Zoned::now().with_time_zone(source.clone());
    let time = query.time.map_or(Ok(now.time()), parse_time)?;
    let date = query.date.map_or(Ok(now.date()), parse_date)?;
    // ponytail: a DST gap or fold resolves with jiff's "compatible" rule (gap:
    // later, fold: earlier). Add a disambiguation field to Query if it matters.
    let at = date
        .to_datetime(time)
        .to_zoned(source.clone())
        .map_err(Error::Moment)?;

    let lead = (!places.iter().any(|p| p.zone == source)).then(|| Place {
        zone: source,
        label: source_label,
    });
    lead.iter()
        .chain(places)
        .map(|place| {
            let local = at.with_time_zone(place.zone.clone());
            let day_shift = at
                .date()
                .until(local.date())
                .map_err(Error::Moment)?
                .get_days();
            Ok(Row {
                label: place.label.clone(),
                band: Band::of_hour(local.hour()),
                is_source: place.zone == *at.time_zone(),
                day_shift,
                at: local,
            })
        })
        .collect()
}

/// Resolves a source zone. The order matters: label, then offset, then IANA.
fn source_zone(input: &str, places: &[Place]) -> Result<TimeZone> {
    if let Some(place) = places.iter().find(|p| p.label.eq_ignore_ascii_case(input)) {
        return Ok(place.zone.clone());
    }
    parse_offset(input).map_or_else(
        || TimeZone::get(input).map_err(|e| Error::Zone(input.to_string(), e)),
        Ok,
    )
}

#[cfg(test)]
mod tests {
    use super::{convert, Query};
    use crate::{band::Band, error::Error, places::parse_places};

    const EVENING: Query = Query {
        time: Some("20:30"),
        date: Some("2026-09-22"),
        zone: Some("GMT+4"),
    };

    #[test]
    fn rows() {
        let places = parse_places("Asia/Tokyo Tokyo; America/New_York New York").unwrap();
        let rows = convert(EVENING, &places).unwrap();
        let tokyo = &rows[1];
        assert_eq!(rows[0].label, "GMT+4");
        assert!(rows[0].is_source && !tokyo.is_source);
        assert_eq!(tokyo.at.strftime("%H:%M %d").to_string(), "01:30 23");
        assert_eq!((tokyo.band, tokyo.day_shift), (Band::Night, 1));
        assert_eq!((rows[2].band, rows[2].day_shift), (Band::Work, 0));
    }

    #[test]
    fn label_is_a_source() {
        let places = parse_places("Asia/Tokyo Tokyo").unwrap();
        let query = Query {
            zone: Some("tokyo"),
            ..EVENING
        };
        let rows = convert(query, &places).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_source);
    }

    #[test]
    fn bad_input() {
        let bad = |query| convert(query, &[]).unwrap_err();
        assert!(matches!(
            bad(Query {
                time: Some("25:00"),
                ..EVENING
            }),
            Error::Time(_)
        ));
        assert!(matches!(
            bad(Query {
                date: Some("soon"),
                ..EVENING
            }),
            Error::Date(_)
        ));
        assert!(matches!(
            bad(Query {
                zone: Some("Mars/Olympus"),
                ..EVENING
            }),
            Error::Zone(..)
        ));
    }
}
