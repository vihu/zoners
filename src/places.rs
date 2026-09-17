//! The configured places: where they come from and how they are written.

use crate::error::{Error, Result};
use jiff::tz::TimeZone;
use std::{env, fs, path::PathBuf};

/// A configured zone and the label it is shown under.
#[derive(Clone, Debug)]
pub struct Place {
    /// Zone the place keeps time in.
    pub zone: TimeZone,
    /// Name shown to the user, also accepted as a source zone.
    pub label: String,
}

const PLACES_VAR: &str = "ZONERS_ZONES";

/// Loads the configured places.
///
/// The `ZONERS_ZONES` environment variable wins, then the file at
/// `$XDG_CONFIG_HOME/zoners/zones` (or `~/.config/zoners/zones`). With
/// neither, a hint goes to stderr and UTC is the only place.
///
/// # Errors
///
/// Returns [Error::Place] if an entry does not start with an IANA zone.
pub fn load_places() -> Result<Vec<Place>> {
    if let Some(text) = env::var(PLACES_VAR).ok().filter(|v| !v.trim().is_empty()) {
        return parse_places(&text);
    }
    let path = config_path();
    let Ok(text) = fs::read_to_string(&path) else {
        eprintln!(
            "hint: no {PLACES_VAR} and no config at {}, showing UTC only",
            path.display()
        );
        return Ok(vec![Place {
            zone: TimeZone::UTC,
            label: "UTC".into(),
        }]);
    };
    parse_places(&text)
}

/// Parses places written one per line or separated by `;`.
///
/// Each entry is `<IANA zone> <label>`, where the label is the rest of the
/// entry and defaults to the zone name. Entries starting with `#` are
/// comments.
///
/// # Errors
///
/// Returns [Error::Place] if an entry does not start with an IANA zone.
pub fn parse_places(text: &str) -> Result<Vec<Place>> {
    text.split(['\n', ';'])
        .map(str::trim)
        .filter(|entry| !entry.is_empty() && !entry.starts_with('#'))
        .map(|entry| {
            let (iana, label) = entry
                .split_once(char::is_whitespace)
                .unwrap_or((entry, entry));
            let zone = TimeZone::get(iana).map_err(|e| Error::Place(entry.to_string(), e))?;
            Ok(Place {
                zone,
                label: label.trim().to_string(),
            })
        })
        .collect()
}

fn config_path() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"))
        .join("zoners/zones")
}

#[cfg(test)]
mod tests {
    use super::parse_places;

    #[test]
    fn entries() {
        let places = parse_places("# team\nUTC Greenwich mean; UTC\n\n").unwrap();
        let labels: Vec<_> = places.iter().map(|p| p.label.as_str()).collect();
        assert_eq!(labels, ["Greenwich mean", "UTC"]);
        assert!(parse_places("Mars/Olympus Base").is_err());
    }
}
