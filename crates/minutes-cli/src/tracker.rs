//! Supported time-tracking backends.

use std::fmt;
use std::str::FromStr;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// A time tracker that can act as a sync source or target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tracker {
    /// Clockify (`https://clockify.me`).
    Clockify,
    /// Harvest (`https://www.getharvest.com`).
    Harvest,
    /// Tempo Timesheets (Jira).
    Tempo,
    /// Timewarrior (local CLI).
    Timewarrior,
    /// Toggl Track.
    Toggl,
}

impl Tracker {
    /// Stable lowercase identifier used in help text and errors.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clockify => "clockify",
            Self::Harvest => "harvest",
            Self::Tempo => "tempo",
            Self::Timewarrior => "timewarrior",
            Self::Toggl => "toggl",
        }
    }
}

impl fmt::Display for Tracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Tracker {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "clockify" => Ok(Self::Clockify),
            "harvest" => Ok(Self::Harvest),
            "tempo" => Ok(Self::Tempo),
            "timewarrior" => Ok(Self::Timewarrior),
            "toggl" => Ok(Self::Toggl),
            other => Err(format!(
                "unknown tracker `{other}`; expected one of: clockify, harvest, tempo, timewarrior, toggl"
            )),
        }
    }
}
