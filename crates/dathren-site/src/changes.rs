use std::collections::{BTreeSet, HashMap};

use dathren::{DatasetMeta, Observation, Period};
use serde::{Deserialize, Serialize};

use crate::chart::format_value;

/// A change below this fraction of the old value is rounding noise, not a revision.
const REVISION_THRESHOLD: f64 = 0.001;
const MAX_EXAMPLES: usize = 3;
pub const MAX_CHANGES: usize = 300;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub date: String,
    pub dataset: String,
    pub title: String,
    pub kind: Kind,
    pub summary: String,
    #[serde(default)]
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    NewData,
    Revised,
    Removed,
}

type Key<'a> = (&'a str, &'a str, Period);

fn key(o: &Observation) -> Key<'_> {
    (&o.measure, &o.geo, o.period)
}

/// Compares two snapshots of the same files. An empty `old` is the first build, so
/// there's nothing to report yet.
pub fn diff(
    meta: &DatasetMeta,
    date: &str,
    old: &[Observation],
    new: &[Observation],
) -> Vec<Change> {
    if old.is_empty() || new.is_empty() {
        return Vec::new();
    }
    let change = |kind, summary: String, examples: Vec<String>| Change {
        date: date.into(),
        dataset: meta.id.into(),
        title: meta.title.into(),
        kind,
        summary,
        examples,
    };
    let unit = |measure: &str| meta.measure(measure).map_or("", |m| m.unit);
    let label = |measure: &str| {
        meta.measure(measure)
            .map_or_else(|| measure.to_string(), |m| m.label.to_string())
    };
    let place = |o: &Observation| o.geo_name.clone().unwrap_or_else(|| o.geo.clone());
    let mut changes = Vec::new();

    let old_periods: BTreeSet<Period> = old.iter().map(|o| o.period).collect();
    let added: BTreeSet<Period> = new
        .iter()
        .map(|o| o.period)
        .filter(|p| !old_periods.contains(p))
        .collect();
    if !added.is_empty() {
        let fresh: Vec<&Observation> = new.iter().filter(|o| added.contains(&o.period)).collect();
        let periods: Vec<String> = added.iter().map(Period::to_string).collect();
        let places: BTreeSet<&str> = fresh.iter().map(|o| o.geo.as_str()).collect();
        let primary = meta.measures.first().map(|m| m.id);
        let mut highlights: Vec<&&Observation> = fresh
            .iter()
            .filter(|o| Some(o.measure.as_str()) == primary && o.best().is_some())
            .collect();
        highlights.sort_by_key(|o| (o.geo != "US", std::cmp::Reverse(o.period)));
        let examples = highlights
            .iter()
            .take(1)
            .map(|o| {
                format!(
                    "{}, {}: {} ({} {})",
                    label(&o.measure),
                    place(o),
                    format_value(o.best().unwrap(), unit(&o.measure)),
                    meta.period_label.to_lowercase(),
                    o.period
                )
            })
            .collect();
        changes.push(change(
            Kind::NewData,
            format!(
                "New data for {} across {} {}.",
                periods.join(", "),
                places.len(),
                if places.len() == 1 { "place" } else { "places" }
            ),
            examples,
        ));
    }

    let old_values: HashMap<Key, Option<f64>> = old.iter().map(|o| (key(o), o.best())).collect();
    let mut revised = Vec::new();
    let mut filled = 0;
    for o in new {
        match (old_values.get(&key(o)), o.best()) {
            (Some(Some(before)), Some(after)) => {
                let delta = (after - before).abs() / before.abs().max(1.0);
                if delta > REVISION_THRESHOLD {
                    revised.push((o, *before, after, delta));
                }
            }
            (Some(None), Some(_)) => filled += 1,
            _ => {}
        }
    }
    if !revised.is_empty() || filled > 0 {
        revised.sort_by(|a, b| b.3.total_cmp(&a.3));
        let mut parts = Vec::new();
        if !revised.is_empty() {
            parts.push(format!(
                "{} earlier {} revised",
                revised.len(),
                plural(revised.len(), "value was", "values were")
            ));
        }
        if filled > 0 {
            parts.push(format!(
                "{filled} previously missing {} now reported",
                plural(filled, "value is", "values are")
            ));
        }
        let examples = revised
            .iter()
            .take(MAX_EXAMPLES)
            .map(|(o, before, after, _)| {
                let u = unit(&o.measure);
                format!(
                    "{}, {}, {}: {} to {}",
                    label(&o.measure),
                    place(o),
                    o.period,
                    format_value(*before, u),
                    format_value(*after, u)
                )
            })
            .collect();
        changes.push(change(
            Kind::Revised,
            format!("{}.", sentence(&parts.join(", and "))),
            examples,
        ));
    }

    let new_keys: BTreeSet<Key> = new.iter().map(key).collect();
    let removed = old.iter().filter(|o| !new_keys.contains(&key(o))).count();
    let newest = new.iter().map(|o| o.period).max();
    let rolled_off = old
        .iter()
        .filter(|o| !new_keys.contains(&key(o)) && Some(o.period) < newest)
        .count();
    if removed > 0 && removed != rolled_off {
        changes.push(change(
            Kind::Removed,
            format!(
                "{removed} {} no longer published upstream.",
                plural(removed, "value is", "values are")
            ),
            Vec::new(),
        ));
    }
    changes
}

fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 { one } else { many }
}

fn sentence(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map_or_else(String::new, |c| c.to_uppercase().chain(chars).collect())
}

/// `YYYY-MM-DD` for a Unix timestamp, without pulling in a date library.
pub fn date_from_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use dathren::registry;

    fn obs(geo: &str, period: Period, value: Option<f64>) -> Observation {
        Observation {
            dataset: "cdc-vsrr-overdose".into(),
            measure: "overdose_deaths".into(),
            geo: geo.into(),
            geo_name: Some(if geo == "US" {
                "United States".into()
            } else {
                geo.into()
            }),
            period,
            value,
            adjusted_value: None,
            ci_low: None,
            ci_high: None,
            flags: Vec::new(),
            note: None,
        }
    }

    fn meta() -> &'static DatasetMeta {
        registry::get("cdc-vsrr-overdose").unwrap().meta()
    }

    #[test]
    fn first_build_reports_nothing() {
        let new = vec![obs("US", Period::month(2026, 5), Some(1000.0))];
        assert!(diff(meta(), "2026-10-25", &[], &new).is_empty());
    }

    #[test]
    fn reports_new_periods_and_revisions() {
        let old = vec![
            obs("US", Period::month(2026, 4), Some(1000.0)),
            obs("WV", Period::month(2026, 4), None),
        ];
        let new = vec![
            obs("US", Period::month(2026, 4), Some(1100.0)),
            obs("WV", Period::month(2026, 4), Some(50.0)),
            obs("US", Period::month(2026, 5), Some(1200.0)),
        ];
        let changes = diff(meta(), "2026-10-25", &old, &new);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].kind, Kind::NewData);
        assert_eq!(changes[0].summary, "New data for 2026-05 across 1 place.");
        assert!(changes[0].examples[0].contains("1,200"));
        assert_eq!(changes[1].kind, Kind::Revised);
        assert_eq!(
            changes[1].summary,
            "1 earlier value was revised, and 1 previously missing value is now reported."
        );
        assert!(changes[1].examples[0].ends_with("1,000 to 1,100"));
    }

    #[test]
    fn ignores_rounding_and_rolled_off_periods() {
        let old = vec![
            obs("US", Period::month(2026, 3), Some(1000.0)),
            obs("US", Period::month(2026, 4), Some(1000.0)),
        ];
        let new = vec![obs("US", Period::month(2026, 4), Some(1000.4))];
        assert!(diff(meta(), "2026-10-25", &old, &new).is_empty());
    }

    #[test]
    fn formats_dates() {
        assert_eq!(date_from_unix(0), "1970-01-01");
        assert_eq!(date_from_unix(1_792_886_400), "2026-10-25");
        assert_eq!(date_from_unix(951_782_400), "2000-02-29");
    }
}
