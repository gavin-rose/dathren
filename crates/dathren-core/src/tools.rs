//! Helpers for working with fetched observations: citations, each place's latest value,
//! suppression-aware totals, and per-capita rates.

use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::{Client, DatasetMeta, Error, Flag, Observation, Query, Result, registry};

pub const DATHREN_URL: &str = "https://dathren.debugr.net";

/// Units that count things, so they can be summed and turned into rates.
pub fn is_count(unit: &str) -> bool {
    matches!(unit, "deaths" | "people")
}

fn unit_of(o: &Observation) -> Option<&'static str> {
    Some(
        registry::get(&o.dataset)
            .ok()?
            .meta()
            .measure(&o.measure)?
            .unit,
    )
}

#[derive(Debug, Serialize)]
pub struct Citation {
    pub apa: String,
    pub bibtex: String,
    pub text: String,
    /// How to cite dathren itself, for methods sections.
    pub dathren: String,
}

/// `accessed` is a `YYYY-MM-DD` date; [`today`] gives the current one.
pub fn cite(meta: &DatasetMeta, accessed: &str) -> Citation {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let mut parts = accessed.splitn(3, '-');
    let (year, month, day) = (
        parts.next().unwrap_or_default(),
        parts
            .next()
            .and_then(|m| m.parse::<usize>().ok())
            .unwrap_or(1),
        parts
            .next()
            .and_then(|d| d.parse::<u32>().ok())
            .unwrap_or(1),
    );
    let month_name = MONTHS[month.clamp(1, 12) - 1];
    let key: String = meta
        .id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    Citation {
        apa: format!(
            "{}. ({year}). {} [Data set]. Retrieved {month_name} {day}, {year}, from {}",
            meta.publisher, meta.title, meta.source_url
        ),
        bibtex: format!(
            "@misc{{{key}{year},\n  author = {{{{{}}}}},\n  title = {{{}}},\n  year = {{{year}}},\n  howpublished = {{\\url{{{}}}}},\n  note = {{Accessed {accessed} via dathren}}\n}}",
            meta.publisher, meta.title, meta.source_url
        ),
        text: format!(
            "{}, {}. {} (accessed {accessed} via dathren, {DATHREN_URL}).",
            meta.publisher, meta.title, meta.source_url
        ),
        dathren: format!(
            "dathren (version {}). Public health data, cleaned up and documented. {DATHREN_URL}",
            env!("CARGO_PKG_VERSION")
        ),
    }
}

/// Today's date as `YYYY-MM-DD`, without pulling in a date library.
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
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

/// The newest observation of each measure for each place. A newer row with no value that
/// was never published loses to an older value, but a suppressed one wins: falling back
/// past a suppression would show a stale number as current.
pub fn latest(observations: impl IntoIterator<Item = Observation>) -> Vec<Observation> {
    let published = |o: &Observation| o.best().is_some() || o.flags.contains(&Flag::Suppressed);
    let mut newest: BTreeMap<(String, String), Observation> = BTreeMap::new();
    for o in observations {
        let key = (o.measure.clone(), o.geo.clone());
        let better = match newest.get(&key) {
            None => true,
            Some(kept) => (published(&o), o.period) > (published(kept), kept.period),
        };
        if better {
            newest.insert(key, o);
        }
    }
    newest.into_values().collect()
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Total {
    /// Sum of the values that were reported.
    pub reported: f64,
    /// `reported` plus the smallest value each suppressed cell could hold.
    pub low: f64,
    /// `reported` plus the largest value each suppressed cell could hold.
    pub high: f64,
    pub reported_count: usize,
    pub suppressed_count: usize,
}

/// Sums counts, treating each suppressed cell as somewhere in `suppressed_range`. NCHS
/// suppresses counts of 1 to 9, so `(1.0, 9.0)` fits most CDC death data.
pub fn total(observations: &[Observation], suppressed_range: (f64, f64)) -> Result<Total> {
    if let Some(o) = observations
        .iter()
        .find(|o| unit_of(o).is_some_and(|u| !is_count(u)))
    {
        return Err(Error::Invalid(format!(
            "`{}` is a rate or percent, not a count, so it can't be summed",
            o.measure
        )));
    }
    let measures: Vec<&str> = {
        let mut m: Vec<&str> = observations.iter().map(|o| o.measure.as_str()).collect();
        m.sort_unstable();
        m.dedup();
        m
    };
    if measures.len() > 1 {
        return Err(Error::Invalid(format!(
            "total needs one measure, got {}; measures can overlap, so adding them double counts",
            measures.join(", ")
        )));
    }
    let mut t = Total {
        reported: 0.0,
        low: 0.0,
        high: 0.0,
        reported_count: 0,
        suppressed_count: 0,
    };
    for o in observations {
        match o.best() {
            Some(v) => {
                t.reported += v;
                t.reported_count += 1;
            }
            None if o.flags.contains(&Flag::Suppressed) => t.suppressed_count += 1,
            None => {}
        }
    }
    t.low = t.reported + suppressed_range.0 * t.suppressed_count as f64;
    t.high = t.reported + suppressed_range.1 * t.suppressed_count as f64;
    Ok(t)
}

/// Turns counts into rates per `per` people using matching-year population. Periods past the
/// newest population estimate use that newest year, noted on the observation. Measures are
/// renamed, e.g. `overdose_deaths` to `overdose_deaths_per_100000`.
pub fn per_capita(
    observations: &[Observation],
    population: &[Observation],
    per: f64,
) -> Result<Vec<Observation>> {
    if let Some(o) = observations
        .iter()
        .find(|o| unit_of(o).is_some_and(|u| !is_count(u)))
    {
        return Err(Error::Invalid(format!(
            "`{}` is already a rate or percent",
            o.measure
        )));
    }
    let mut people: HashMap<&str, BTreeMap<i32, f64>> = HashMap::new();
    for p in population {
        if let Some(v) = p.value {
            people
                .entry(p.geo.as_str())
                .or_default()
                .insert(p.period.year, v);
        }
    }
    Ok(observations
        .iter()
        .map(|o| {
            let mut rate = o.clone();
            rate.measure = format!("{}_per_{}", o.measure, per as i64);
            let years = people.get(o.geo.as_str());
            let (year, pop) =
                match years.and_then(|y| y.get(&o.period.year).map(|p| (o.period.year, *p))) {
                    Some(found) => found,
                    None => match years
                        .and_then(|y| y.last_key_value())
                        .filter(|(y, _)| **y < o.period.year)
                    {
                        Some((y, p)) => (*y, *p),
                        None => {
                            rate.value = None;
                            rate.adjusted_value = None;
                            rate.note = Some(format!(
                                "No population estimate for {} in {}",
                                o.geo, o.period.year
                            ));
                            return rate;
                        }
                    },
                };
            let scale = |v: Option<f64>| v.map(|v| v / pop * per);
            rate.value = scale(o.value);
            rate.adjusted_value = scale(o.adjusted_value);
            rate.ci_low = scale(o.ci_low);
            rate.ci_high = scale(o.ci_high);
            rate.note = Some(format!(
                "Per {} people, using {year} Census population",
                grouped(per as i64)
            ));
            rate
        })
        .collect())
}

/// [`per_capita`], fetching Census population for the places in `observations`.
pub fn per_capita_fetch(
    client: &Client,
    observations: &[Observation],
    per: f64,
) -> Result<Vec<Observation>> {
    let population = registry::get("census-population")?.fetch(client, &Query::default())?;
    per_capita(observations, &population, per)
}

fn grouped(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// How many observations carry each flag, for warnings.
pub fn flag_counts(observations: &[Observation]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for flag in observations.iter().flat_map(|o| &o.flags) {
        *counts.entry(flag.as_str()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Period;

    fn obs(
        dataset: &str,
        measure: &str,
        geo: &str,
        year: i32,
        value: Option<f64>,
        flags: Vec<Flag>,
    ) -> Observation {
        Observation {
            dataset: dataset.into(),
            measure: measure.into(),
            geo: geo.into(),
            geo_name: None,
            period: Period::year(year),
            value,
            adjusted_value: None,
            ci_low: None,
            ci_high: None,
            flags,
            note: None,
        }
    }

    const COUNTY: &str = "cdc-vsrr-county-overdose";

    #[test]
    fn totals_bound_suppressed_cells() {
        let rows = vec![
            obs(COUNTY, "overdose_deaths", "01001", 2026, Some(40.0), vec![]),
            obs(
                COUNTY,
                "overdose_deaths",
                "01003",
                2026,
                None,
                vec![Flag::Suppressed],
            ),
            obs(
                COUNTY,
                "overdose_deaths",
                "01005",
                2026,
                None,
                vec![Flag::Suppressed],
            ),
        ];
        let t = total(&rows, (1.0, 9.0)).unwrap();
        assert_eq!(
            (t.reported, t.low, t.high, t.suppressed_count),
            (40.0, 42.0, 58.0, 2)
        );
    }

    #[test]
    fn totals_refuse_rates_and_mixed_measures() {
        let rate = vec![obs(
            "nchs-mortality-rates",
            "suicide",
            "US",
            2025,
            Some(13.7),
            vec![],
        )];
        assert!(total(&rate, (1.0, 9.0)).is_err());
        let mixed = vec![
            obs("cdc-vsrr-overdose", "heroin", "US", 2025, Some(1.0), vec![]),
            obs(
                "cdc-vsrr-overdose",
                "cocaine",
                "US",
                2025,
                Some(1.0),
                vec![],
            ),
        ];
        assert!(total(&mixed, (1.0, 9.0)).is_err());
    }

    #[test]
    fn per_capita_uses_matching_or_newest_year() {
        let deaths = vec![
            obs(COUNTY, "overdose_deaths", "01001", 2024, Some(50.0), vec![]),
            obs(COUNTY, "overdose_deaths", "01001", 2026, Some(60.0), vec![]),
            obs(COUNTY, "overdose_deaths", "01001", 2019, Some(30.0), vec![]),
        ];
        let pop = vec![
            obs(
                "census-population",
                "population",
                "01001",
                2024,
                Some(50_000.0),
                vec![],
            ),
            obs(
                "census-population",
                "population",
                "01001",
                2025,
                Some(60_000.0),
                vec![],
            ),
        ];
        let rates = per_capita(&deaths, &pop, 100_000.0).unwrap();
        assert_eq!(rates[0].measure, "overdose_deaths_per_100000");
        assert_eq!(rates[0].value, Some(100.0));
        assert_eq!(rates[1].value, Some(100.0));
        assert_eq!(
            rates[1].note.as_deref(),
            Some("Per 100,000 people, using 2025 Census population")
        );
        assert_eq!(rates[2].value, None);
    }

    #[test]
    fn latest_skips_unpublished_but_keeps_suppressed() {
        let kept = latest(vec![
            obs("x", "m", "TN", 2023, Some(15.0), vec![]),
            obs("x", "m", "TN", 2024, None, vec![]),
            obs("x", "m", "WV", 2023, Some(20.0), vec![]),
            obs("x", "m", "WV", 2024, None, vec![Flag::Suppressed]),
        ]);
        let get = |geo: &str| kept.iter().find(|o| o.geo == geo).unwrap();
        assert_eq!(get("TN").period.year, 2023);
        assert_eq!(get("WV").period.year, 2024);
    }

    #[test]
    fn cites_publisher_and_access_date() {
        let meta = registry::get("cdc-vsrr-overdose").unwrap().meta();
        let c = cite(meta, "2026-10-29");
        assert!(c.apa.starts_with("CDC National Center for Health Statistics (VSRR). (2026). Provisional Drug Overdose Death Counts [Data set]. Retrieved October 29, 2026, from https://"));
        assert!(c.bibtex.starts_with("@misc{cdcvsrroverdose2026,"));
        assert!(
            c.bibtex
                .contains("note = {Accessed 2026-10-29 via dathren}")
        );
    }
}
