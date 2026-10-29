//! YRBSS mental health indicators for high school students, nationally and by state.
//! <https://data.cdc.gov/d/nu3s-3dwd>

use serde::Deserialize;

use super::number;
use super::socrata::{self, quote};
use crate::{
    Caveat, Client, DatasetMeta, Geography, Measure, Observation, Period, Query, Result, Source,
};

pub struct YrbssMentalHealth;

const ID: &str = "yrbss-mental-health";

/// Measure ids paired with how the upstream `question` text begins.
const QUESTIONS: &[(&str, &str)] = &[
    (
        "persistent_sadness",
        "During the past 12 months, did you ever feel so sad or hopeless",
    ),
    (
        "considered_suicide",
        "During the past 12 months, did you ever seriously consider attempting suicide",
    ),
    (
        "suicide_plan",
        "During the past 12 months, did you make a plan about how you would attempt suicide",
    ),
    (
        "attempted_suicide",
        "During the past 12 months, how many times did you actually attempt suicide",
    ),
    (
        "poor_mental_health",
        "During the past 30 days, how often was your mental health not good",
    ),
    (
        "rx_opioid_misuse",
        "Ever took prescription pain medicine without a doctor's prescription",
    ),
    ("illicit_drugs", "Ever used select illicit drugs"),
];

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Youth Mental Health Indicators",
    publisher: "CDC Youth Risk Behavior Surveillance System (YRBSS)",
    summary: "High school students reporting persistent sadness, suicidal thoughts and attempts, \
              poor mental health, and drug misuse, nationally and by state.",
    source_url: "https://www.cdc.gov/yrbs/",
    update_cadence: "Every two years",
    topics: &["mental health", "youth", "suicide", "survey"],
    geographies: &[Geography::Nation, Geography::State],
    period_label: "Year",
    series_note: "Percent of high school students, with 95% confidence intervals.",
    measures: &[
        Measure { id: "persistent_sadness", label: "Persistent sadness or hopelessness (2+ weeks, past year)", code: None, unit: "percent" },
        Measure { id: "considered_suicide", label: "Seriously considered suicide (past year)", code: None, unit: "percent" },
        Measure { id: "suicide_plan", label: "Made a suicide plan (past year)", code: None, unit: "percent" },
        Measure { id: "attempted_suicide", label: "Attempted suicide (past year)", code: None, unit: "percent" },
        Measure { id: "poor_mental_health", label: "Poor mental health most of the time or always (past 30 days)", code: None, unit: "percent" },
        Measure { id: "rx_opioid_misuse", label: "Ever misused prescription pain medicine", code: None, unit: "percent" },
        Measure { id: "illicit_drugs", label: "Ever used select illicit drugs", code: None, unit: "percent" },
    ],
    caveats: &[
        Caveat {
            title: "The survey runs every two years",
            detail: "Results cover odd years (2019, 2021, 2023). Nothing new arrives in between.",
        },
        Caveat {
            title: "Values are survey estimates",
            detail: "Each value comes with a 95% confidence interval in `ci_low` and `ci_high`. \
                     Students who aren't in school aren't surveyed.",
        },
        Caveat {
            title: "Not every state takes part",
            detail: "State coverage changes by year, and some states ask only some questions. A \
                     missing state means no data, not zero.",
        },
        Caveat {
            title: "New York's numbers exclude New York City",
            detail: "Most New York results come as `NYA`, the state outside New York City, so \
                     they aren't statewide. A few questions also have a statewide `NY` value.",
        },
        Caveat {
            title: "Poor mental health was added in 2021",
            detail: "There's no 2019 value for `poor_mental_health`, so its trend starts in 2021.",
        },
    ],
};

#[derive(Deserialize)]
struct Row {
    year: String,
    question: String,
    area: String,
    area_abbr: String,
    percent: Option<String>,
    low_confidence_interval: Option<String>,
    high_confidence_interval: Option<String>,
}

impl Source for YrbssMentalHealth {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        META.check_measures(&query.measures)?;
        let mut clauses = vec![format!("demographics_type = {}", quote("Total"))];
        match query.geo.as_deref().map(str::to_uppercase).as_deref() {
            Some("US") => clauses.push(format!("area_abbr = {}", quote("XX"))),
            Some("AZ") => clauses.push(format!("area_abbr in ({}, {})", quote("AZ"), quote("AZB"))),
            Some(geo) => clauses.push(format!("area_abbr = {}", quote(geo))),
            None => {}
        }
        if let Some(year) = query.since_year {
            clauses.push(format!("year >= {}", quote(&year.to_string())));
        }

        let rows: Vec<Row> =
            socrata::fetch_all(client, "data.cdc.gov", "nu3s-3dwd", &clauses.join(" AND "))?;
        let mut observations: Vec<Observation> = rows
            .into_iter()
            .filter_map(normalize)
            .filter(|o| query.measures.is_empty() || query.measures.contains(&o.measure))
            .collect();
        observations
            .sort_by(|a, b| (&a.measure, &a.geo, a.period).cmp(&(&b.measure, &b.geo, b.period)));
        Ok(observations)
    }
}

fn normalize(row: Row) -> Option<Observation> {
    let measure = QUESTIONS
        .iter()
        .find(|(_, start)| row.question.starts_with(start))?
        .0;
    let (geo, geo_name) = match row.area_abbr.as_str() {
        "XX" => ("US".to_string(), "United States".to_string()),
        "AZB" => ("AZ".to_string(), row.area),
        _ => (row.area_abbr, row.area),
    };
    Some(Observation {
        dataset: ID.into(),
        measure: measure.into(),
        geo,
        geo_name: Some(geo_name),
        period: Period::year(row.year.parse().ok()?),
        value: number(row.percent.as_deref()),
        adjusted_value: None,
        ci_low: number(row.low_confidence_interval.as_deref()),
        ci_high: number(row.high_confidence_interval.as_deref()),
        flags: Vec::new(),
        note: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(abbr: &str, area: &str) -> Row {
        Row {
            year: "2023".into(),
            question:
                "During the past 12 months, did you ever seriously consider attempting suicide?"
                    .into(),
            area: area.into(),
            area_abbr: abbr.into(),
            percent: Some("20.4".into()),
            low_confidence_interval: Some("19.1".into()),
            high_confidence_interval: Some("21.8".into()),
        }
    }

    #[test]
    fn maps_area_codes() {
        let us = normalize(row("XX", "United States")).unwrap();
        assert_eq!(
            (us.geo.as_str(), us.measure.as_str()),
            ("US", "considered_suicide")
        );
        assert_eq!(normalize(row("AZB", "Arizona")).unwrap().geo, "AZ");
        let nya = normalize(row("NYA", "New York (Excluding New York City)")).unwrap();
        assert_eq!(nya.geo, "NYA");
        assert_eq!(nya.ci_high, Some(21.8));
    }
}
