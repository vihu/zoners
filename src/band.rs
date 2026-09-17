//! Parts of the day used to colour a local hour.

use std::{fmt, ops::Range};

/// Part of the day a local hour falls in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Band {
    /// Office hours.
    Work,
    /// Up, but outside office hours.
    Awake,
    /// Probably asleep.
    Night,
}

const WORK_HOURS: Range<i8> = 9..17;
const AWAKE_HOURS: Range<i8> = 7..22;

impl Band {
    /// Returns the band a local hour in `0..24` falls in.
    #[must_use]
    pub fn of_hour(hour: i8) -> Self {
        match hour {
            h if WORK_HOURS.contains(&h) => Self::Work,
            h if AWAKE_HOURS.contains(&h) => Self::Awake,
            _ => Self::Night,
        }
    }

    /// Returns the lowercase name, as shown to the user.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::Awake => "awake",
            Self::Night => "night",
        }
    }
}

impl fmt::Display for Band {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::Band;

    #[test]
    fn hours() {
        for (hour, want) in [
            (6, Band::Night),
            (7, Band::Awake),
            (9, Band::Work),
            (16, Band::Work),
            (17, Band::Awake),
            (22, Band::Night),
        ] {
            assert_eq!(Band::of_hour(hour), want, "{hour}");
        }
    }
}
