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
    pub geographies: &'static [Geography],
    /// How a period reads in prose, e.g. "12 months ending" before `2026-05`.
    pub period_label: &'static str,
    pub series_note: &'static str,
    pub measures: &'static [Measure],
    pub caveats: &'static [Caveat],
}

impl DatasetMeta {
    pub fn measure(&self, id: &str) -> Option<&Measure> {
        self.measures.iter().find(|m| m.id == id)
    }

    pub fn has(&self, geography: Geography) -> bool {
        self.geographies.contains(&geography)
    }

    pub fn check_measures(&self, ids: &[String]) -> crate::Result<()> {
        match ids.iter().find(|id| self.measure(id).is_none()) {
            Some(id) => Err(crate::Error::UnknownMeasure {
                dataset: self.id.into(),
                measure: id.clone(),
            }),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Geography {
    Nation,
    State,
    County,
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

/// A month, a quarter, or a whole year when both are `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Period {
    pub year: i32,
    pub month: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quarter: Option<u8>,
}

impl Period {
    pub fn year(year: i32) -> Self {
        Self {
            year,
            month: None,
            quarter: None,
        }
    }

    pub fn month(year: i32, month: u8) -> Self {
        Self {
            year,
            month: Some(month),
            quarter: None,
        }
    }

    pub fn quarter(year: i32, quarter: u8) -> Self {
        Self {
            year,
            month: None,
            quarter: Some(quarter),
        }
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.month, self.quarter) {
            (Some(m), _) => write!(f, "{}-{m:02}", self.year),
            (None, Some(q)) => write!(f, "{}-Q{q}", self.year),
            (None, None) => write!(f, "{}", self.year),
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
    /// Readable name for `geo` when the code alone isn't, e.g. a county FIPS code.
    pub geo_name: Option<String>,
    pub period: Period,
    pub value: Option<f64>,
    /// Upstream's own correction for known reporting gaps, when it publishes one.
    pub adjusted_value: Option<f64>,
    /// 95% confidence interval, for survey and model-based estimates.
    pub ci_low: Option<f64>,
    pub ci_high: Option<f64>,
    pub flags: Vec<Flag>,
    pub note: Option<String>,
}

impl Observation {
    pub fn best(&self) -> Option<f64> {
        self.adjusted_value.or(self.value)
    }
}
