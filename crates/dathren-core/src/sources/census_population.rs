//! US Census Bureau population estimates for the nation, states, and counties.
//! <https://www.census.gov/programs-surveys/popest.html>

use std::collections::BTreeMap;

use crate::geo::{self, county_fips};
use crate::{
    Caveat, Client, DatasetMeta, Error, Geography, Measure, Observation, Period, Query, Result,
    Source,
};

pub struct CensusPopulation;

const ID: &str = "census-population";
const BASE: &str = "https://www2.census.gov/programs-surveys/popest/datasets";
const DECADE: i32 = 2020;

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Population Estimates",
    publisher: "US Census Bureau Population Estimates Program",
    summary: "Official yearly population estimates for the nation, every state, and every \
              county since 2020. Use them to turn counts into rates.",
    source_url: "https://www.census.gov/programs-surveys/popest.html",
    update_cadence: "Yearly",
    topics: &["population", "denominators"],
    geographies: &[Geography::Nation, Geography::State, Geography::County],
    period_label: "Year",
    series_note: "Resident population on July 1 of each year.",
    measures: &[Measure { id: "population", label: "Resident population", code: None, unit: "people" }],
    caveats: &[
        Caveat {
            title: "Each release revises earlier years",
            detail: "Every new vintage re-estimates all years back to 2020. Dathren always uses \
                     the newest vintage.",
        },
        Caveat {
            title: "Estimates are for July 1",
            detail: "Values are mid-year estimates, which is what most published rates use as \
                     their denominator.",
        },
        Caveat {
            title: "Connecticut uses planning regions",
            detail: "Since 2022, Connecticut is reported by its nine planning regions instead of \
                     its eight former counties, matching CDC county data.",
        },
    ],
};

impl Source for CensusPopulation {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        META.check_measures(&query.measures)?;
        let (vintage, text) = newest_vintage(client)?;
        let mut observations = parse(&text, vintage);
        let mut national: BTreeMap<i32, f64> = BTreeMap::new();
        for o in observations.iter().filter(|o| o.geo.len() == 2) {
            *national.entry(o.period.year).or_default() += o.value.unwrap_or_default();
        }
        observations.extend(
            national
                .into_iter()
                .map(|(year, total)| observation("US".into(), "United States".into(), year, total)),
        );

        let wanted = query.geo.as_deref().map(str::to_uppercase);
        observations.retain(|o| {
            wanted.as_deref().is_none_or(|g| {
                o.geo == g
                    || (g.len() == 2
                        && g != "US"
                        && o.geo.len() == 5
                        && geo::state(g).is_some_and(|s| o.geo.starts_with(s.fips)))
            }) && query.since_year.is_none_or(|y| o.period.year >= y)
        });
        observations.sort_by(|a, b| (&a.geo, a.period).cmp(&(&b.geo, b.period)));
        Ok(observations)
    }
}

/// Census publishes one file per vintage; the newest may not exist yet early in a year.
fn newest_vintage(client: &Client) -> Result<(i32, String)> {
    let this_year = 1970
        + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
            / 31_556_952) as i32;
    let mut last_error = None;
    for vintage in (DECADE + 1..=this_year).rev().take(3) {
        let url = format!("{BASE}/{DECADE}-{vintage}/counties/totals/co-est{vintage}-alldata.csv");
        match client
            .http
            .get(&url)
            .send()
            .and_then(|r| r.error_for_status())
        {
            Ok(response) => {
                let bytes = response.bytes()?;
                return Ok((vintage, bytes.iter().map(|&b| b as char).collect()));
            }
            Err(e) => last_error = Some(e),
        }
    }
    Err(Error::Http(
        last_error.expect("at least one vintage was tried"),
    ))
}

fn parse(text: &str, vintage: i32) -> Vec<Observation> {
    let mut reader = csv::Reader::from_reader(text.as_bytes());
    let Ok(headers) = reader.headers().cloned() else {
        return Vec::new();
    };
    let column = |name: &str| headers.iter().position(|h| h == name);
    let (Some(level), Some(state), Some(county), Some(name)) = (
        column("SUMLEV"),
        column("STATE"),
        column("COUNTY"),
        column("CTYNAME"),
    ) else {
        return Vec::new();
    };
    let years: Vec<(i32, usize)> = (DECADE..=vintage)
        .filter_map(|y| Some((y, column(&format!("POPESTIMATE{y}"))?)))
        .collect();

    let mut observations = Vec::new();
    for record in reader.records().flatten() {
        let (geo, geo_name) = match &record[level] {
            "040" => {
                let Some(s) = geo::STATES.iter().find(|s| s.fips == &record[state]) else {
                    continue;
                };
                (s.abbr.to_string(), s.name.to_string())
            }
            "050" => {
                let fips = county_fips(&format!("{}{}", &record[state], &record[county]));
                let abbr = geo::STATES
                    .iter()
                    .find(|s| s.fips == &record[state])
                    .map_or("", |s| s.abbr);
                (fips, format!("{}, {abbr}", &record[name]))
            }
            _ => continue,
        };
        for &(year, i) in &years {
            if let Ok(value) = record[i].parse::<f64>() {
                observations.push(observation(geo.clone(), geo_name.clone(), year, value));
            }
        }
    }
    observations
}

fn observation(geo: String, geo_name: String, year: i32, value: f64) -> Observation {
    Observation {
        dataset: ID.into(),
        measure: "population".into(),
        geo,
        geo_name: Some(geo_name),
        period: Period::year(year),
        value: Some(value),
        adjusted_value: None,
        ci_low: None,
        ci_high: None,
        flags: Vec::new(),
        note: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_states_and_counties() {
        let csv = "SUMLEV,STATE,COUNTY,STNAME,CTYNAME,POPESTIMATE2020,POPESTIMATE2021\n\
                   040,35,000,New Mexico,New Mexico,2117000,2116000\n\
                   050,35,013,New Mexico,Do\u{f1}a Ana County,219000,220000\n";
        let obs = parse(csv, 2021);
        assert_eq!(obs.len(), 4);
        assert_eq!((obs[0].geo.as_str(), obs[0].value), ("NM", Some(2117000.0)));
        assert_eq!(obs[3].geo, "35013");
        assert_eq!(obs[3].geo_name.as_deref(), Some("Do\u{f1}a Ana County, NM"));
        assert_eq!(obs[3].period.to_string(), "2021");
    }
}
