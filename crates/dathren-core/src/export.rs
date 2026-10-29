use std::io::Write;

use serde::{Deserialize, Serialize};

use crate::{Flag, Observation};

/// An observation flattened for tables: the period as text and flags as names.
#[derive(Debug, Serialize)]
pub struct Record<'a> {
    pub dataset: &'a str,
    pub measure: &'a str,
    pub geo: &'a str,
    pub geo_name: Option<&'a str>,
    pub period: String,
    pub value: Option<f64>,
    pub adjusted_value: Option<f64>,
    pub ci_low: Option<f64>,
    pub ci_high: Option<f64>,
    pub flags: Vec<&'static str>,
    pub note: Option<&'a str>,
}

impl<'a> From<&'a Observation> for Record<'a> {
    fn from(o: &'a Observation) -> Self {
        Record {
            dataset: &o.dataset,
            measure: &o.measure,
            geo: &o.geo,
            geo_name: o.geo_name.as_deref(),
            period: o.period.to_string(),
            value: o.value,
            adjusted_value: o.adjusted_value,
            ci_low: o.ci_low,
            ci_high: o.ci_high,
            flags: o.flags.iter().map(|f| f.as_str()).collect(),
            note: o.note.as_deref(),
        }
    }
}

pub fn records(observations: &[Observation]) -> Vec<Record<'_>> {
    observations.iter().map(Record::from).collect()
}

/// A [`Record`] coming back from Python or JavaScript.
#[derive(Debug, Deserialize)]
struct Incoming {
    dataset: String,
    measure: String,
    geo: String,
    geo_name: Option<String>,
    period: String,
    value: Option<f64>,
    adjusted_value: Option<f64>,
    #[serde(default)]
    ci_low: Option<f64>,
    #[serde(default)]
    ci_high: Option<f64>,
    #[serde(default)]
    flags: Vec<String>,
    note: Option<String>,
}

/// Parses records produced by [`records`] back into observations.
pub fn from_records_json(json: &str) -> crate::Result<Vec<Observation>> {
    let rows: Vec<Incoming> = serde_json::from_str(json)
        .map_err(|e| crate::Error::Invalid(format!("rows don't look like dathren records: {e}")))?;
    rows.into_iter()
        .map(|r| {
            Ok(Observation {
                period: r.period.parse()?,
                flags: r.flags.iter().filter_map(|f| Flag::from_name(f)).collect(),
                dataset: r.dataset,
                measure: r.measure,
                geo: r.geo,
                geo_name: r.geo_name,
                value: r.value,
                adjusted_value: r.adjusted_value,
                ci_low: r.ci_low,
                ci_high: r.ci_high,
                note: r.note,
            })
        })
        .collect()
}

pub fn write_csv<W: Write>(w: W, observations: &[Observation]) -> csv::Result<()> {
    let mut out = csv::Writer::from_writer(w);
    out.write_record([
        "dataset",
        "measure",
        "geo",
        "geo_name",
        "period",
        "value",
        "adjusted_value",
        "ci_low",
        "ci_high",
        "flags",
        "note",
    ])?;
    let num = |v: Option<f64>| v.map(|v| v.to_string()).unwrap_or_default();
    for r in records(observations) {
        out.write_record([
            r.dataset,
            r.measure,
            r.geo,
            r.geo_name.unwrap_or_default(),
            &r.period,
            &num(r.value),
            &num(r.adjusted_value),
            &num(r.ci_low),
            &num(r.ci_high),
            &r.flags.join(";"),
            r.note.unwrap_or_default(),
        ])?;
    }
    out.flush()?;
    Ok(())
}
