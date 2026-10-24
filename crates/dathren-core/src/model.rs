use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct DatasetMeta {
    pub id: &'static str,
    pub title: &'static str,
    pub publisher: &'static str,
    pub summary: &'static str,
    pub source_url: &'static str,
    pub update_cadence: &'static str,
    pub topics: &'static [&'static str],
    pub measures: &'static [Measure],
    pub caveats: &'static [Caveat],
}

impl DatasetMeta {
    pub fn measure(&self, id: &str) -> Option<&Measure> {
        self.measures.iter().find(|m| m.id == id)
    }
}

#[derive(Debug, Serialize)]
pub struct Measure {
    pub id: &'static str,
    pub label: &'static str,
    /// Upstream classification code, e.g. an ICD-10 range.
    pub code: Option<&'static str>,
    pub unit: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Caveat {
    pub title: &'static str,
    pub detail: &'static str,
}

/// A month, or a whole year when `month` is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Period {
    pub year: i32,
    pub month: Option<u8>,
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.month {
            Some(m) => write!(f, "{}-{m:02}", self.year),
            None => write!(f, "{}", self.year),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flag {
    /// Withheld upstream for privacy or data quality; `value` is `None`.
    Suppressed,
    /// Reporting is still incomplete for this period.
    Underreported,
    /// May be revised when final data is released.
    Provisional,
}

impl Flag {
    pub fn as_str(self) -> &'static str {
        match self {
            Flag::Suppressed => "suppressed",
            Flag::Underreported => "underreported",
            Flag::Provisional => "provisional",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub dataset: String,
    pub measure: String,
    pub geo: String,
    pub period: Period,
    pub value: Option<f64>,
    /// Upstream's own correction for known reporting gaps, when it publishes one.
    pub adjusted_value: Option<f64>,
    pub flags: Vec<Flag>,
    pub note: Option<String>,
}

impl Observation {
    pub fn best(&self) -> Option<f64> {
        self.adjusted_value.or(self.value)
    }
}
