//! NHIS mental health indicators for US adults, national only.
//! <https://data.cdc.gov/d/d89q-62iu>

use serde::Deserialize;

use super::number;
use super::socrata::{self, quote};
use crate::{
    Caveat, Client, DatasetMeta, Geography, Measure, Observation, Period, Query, Result, Source,
};

pub struct NhisMentalHealth;

const ID: &str = "nhis-mental-health";

/// Measure ids paired with how the upstream `question` text begins.
const QUESTIONS: &[(&str, &str)] = &[
    (
        "anxiety_symptoms",
        "Composite measure of frequency and intensity of feelings of anxiety",
    ),
    (
        "depression_symptoms",
        "Composite measure of frequency and intensity of feelings of depression",
    ),
    (
        "anxiety_diagnosis",
        "Have you EVER been told by a doctor or other health professional that you had any type of anxiety",
    ),
    (
        "depression_diagnosis",
        "Have you EVER been told by a doctor or other health professional that you had any type of depression",
    ),
    (
        "counseling",
        "During the past 12 months, did you receive counseling or therapy",
    ),
    (
        "social_limitation",
        "Because of a physical mental, or emotional condition",
    ),
    (
        "emotional_support",
        "How often do you get the social and emotional support",
    ),
    (
        "life_satisfaction",
        "In general, how satisfied are you with your life",
    ),
];

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Adult Mental Health Indicators, National",
    publisher: "CDC National Health Interview Survey (NHIS)",
    summary: "Yearly national estimates of anxiety and depression symptoms, diagnoses, \
              counseling, and life satisfaction among US adults since 2019.",
    source_url: "https://www.cdc.gov/nchs/nhis/",
    update_cadence: "Yearly",
    topics: &["mental health", "survey", "treatment"],
    geographies: &[Geography::Nation],
    period_label: "Year",
    series_note: "Percent of adults, with 95% confidence intervals.",
    measures: &[
        Measure { id: "anxiety_symptoms", label: "Frequent, intense feelings of anxiety", code: None, unit: "percent" },
        Measure { id: "depression_symptoms", label: "Frequent, intense feelings of depression", code: None, unit: "percent" },
        Measure { id: "anxiety_diagnosis", label: "Ever told they have an anxiety disorder", code: None, unit: "percent" },
        Measure { id: "depression_diagnosis", label: "Ever told they have depression", code: None, unit: "percent" },
        Measure { id: "counseling", label: "Received counseling or therapy (past year)", code: None, unit: "percent" },
        Measure { id: "social_limitation", label: "Difficulty with social activities due to a health condition", code: None, unit: "percent" },
        Measure { id: "emotional_support", label: "Usually or always get the social and emotional support they need", code: None, unit: "percent" },
        Measure { id: "life_satisfaction", label: "Satisfied with their life", code: None, unit: "percent" },
    ],
    caveats: &[
        Caveat {
            title: "National only",
            detail: "NHIS isn't designed for state estimates. For states, see the BRFSS and \
                     YRBSS mental health datasets.",
        },
        Caveat {
            title: "Symptom measures combine two questions",
            detail: "Anxiety and depression symptoms combine how often adults feel that way and \
                     how strongly. Check CDC's definitions before comparing them with other surveys.",
        },
        Caveat {
            title: "2020 was collected differently",
            detail: "The pandemic moved interviews to the phone for part of 2020 and lowered \
                     response, so compare 2020 with other years carefully.",
        },
        Caveat {
            title: "Some questions weren't asked every year",
            detail: "Emotional support covers 2020 and 2021, and life satisfaction starts in \
                     2021. The teen survey isn't included because it pools July 2021 to December \
                     2023 into one estimate.",
        },
    ],
};

#[derive(Deserialize)]
struct Row {
    year: String,
    question: String,
    data_source: Option<String>,
    percent: Option<String>,
    low_confidence_interval: Option<String>,
    high_confidence_interval: Option<String>,
}

impl Source for NhisMentalHealth {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        META.check_measures(&query.measures)?;
        if query
            .geo
            .as_deref()
            .is_some_and(|g| !g.eq_ignore_ascii_case("US"))
        {
            return Ok(Vec::new());
        }
        let mut clauses = vec![
            format!("demographics_type = {}", quote("Total")),
            format!("data_source = {}", quote("NHIS")),
        ];
        if let Some(year) = query.since_year {
            clauses.push(format!("year >= {}", quote(&year.to_string())));
        }

        let rows: Vec<Row> =
            socrata::fetch_all(client, "data.cdc.gov", "d89q-62iu", &clauses.join(" AND "))?;
        let mut observations: Vec<Observation> = rows
            .into_iter()
            .filter_map(normalize)
            .filter(|o| query.measures.is_empty() || query.measures.contains(&o.measure))
            .collect();
        observations.sort_by(|a, b| (&a.measure, a.period).cmp(&(&b.measure, b.period)));
        Ok(observations)
    }
}

fn normalize(row: Row) -> Option<Observation> {
    if row.data_source.as_deref() != Some("NHIS") {
        return None;
    }
    let measure = QUESTIONS
        .iter()
        .find(|(_, start)| row.question.starts_with(start))?
        .0;
    Some(Observation {
        dataset: ID.into(),
        measure: measure.into(),
        geo: "US".into(),
        geo_name: Some("United States".into()),
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

    fn row(source: &str, year: &str) -> Row {
        Row {
            year: year.into(),
            question: "During the past 12 months, did you receive counseling or therapy from a mental health professional?".into(),
            data_source: Some(source.into()),
            percent: Some("14.0".into()),
            low_confidence_interval: Some("13.4".into()),
            high_confidence_interval: Some("14.6".into()),
        }
    }

    #[test]
    fn keeps_adult_survey_years() {
        let obs = normalize(row("NHIS", "2024")).unwrap();
        assert_eq!(
            (obs.measure.as_str(), obs.period.to_string()),
            ("counseling", "2024".to_string())
        );
        assert!(normalize(row("NHIS-Teen", "July 2021 - December 2023")).is_none());
    }
}
