//! Error type shared by the library and both binaries.

use std::fmt;

/// Result type for this crate.
pub type Result<T = ()> = std::result::Result<T, Error>;

/// Error type for this crate.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A time input that is not a clock time.
    Time(String),
    /// A date input that is not `YYYY-MM-DD`.
    Date(String),
    /// A zone input that is not a label, an offset or an IANA name.
    Zone(String, jiff::Error),
    /// A config entry whose first word is not an IANA zone.
    Place(String, jiff::Error),
    /// A moment that cannot be represented in the source zone.
    Moment(jiff::Error),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Time(_) | Error::Date(_) => None,
            Error::Zone(_, source) | Error::Place(_, source) | Error::Moment(source) => {
                Some(source)
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Time(input) => {
                write!(f, "time must look like 20:30, 8.30pm or 8pm, got {input:?}")
            }
            Error::Date(input) => write!(f, "date must look like 2026-09-22, got {input:?}"),
            Error::Zone(input, _) => write!(
                f,
                "zone must be a config label, an offset like GMT+4 or an IANA name, got {input:?}"
            ),
            Error::Place(entry, _) => {
                write!(
                    f,
                    "config entry must start with an IANA zone, got {entry:?}"
                )
            }
            Error::Moment(_) => write!(f, "moment cannot be represented in the source zone"),
        }
    }
}
