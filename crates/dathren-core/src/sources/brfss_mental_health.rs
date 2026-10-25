//! BRFSS mental health indicators by state.
//! <https://data.cdc.gov/d/5eh7-pjx8>

use serde::Deserialize;

use super::number;
use super::socrata::{self, quote};
use crate::{
    Caveat, Client, DatasetMeta, Geography, Measure, Observation, Period, Query, Result, Source,
};

pub struct BrfssMentalHealth;

const ID: &str = "brfss-mental-health";

/// Measure ids paired with how the upstream `question` text begins.
const QUESTIONS: &[(&str, &str)] = &[
    ("mental_distress", "Now thinking about your mental health"),
    (
        "physical_distress",
        "Now thinking about your physical health",
    ),
    (
        "activity_limitation",
        "During the past 30 days, how many days did poor physical or mental health",
    ),
    (
        "depression",
        "Have you ever been told you have a depressive disorder",
    ),
    ("loneliness", "How often do you feel socially isolated"),
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
    title: "Mental Health Indicators by State",
    publisher: "CDC Behavioral Risk Factor Surveillance System (BRFSS)",
    summary: "Survey estimates of frequent mental distress, depression, loneliness, emotional \
              support, and life satisfaction among adults in each state.",
    source_url: "https://www.cdc.gov/mental-health/data-research/",
    update_cadence: "Yearly",
    topics: &["mental health", "survey"],
    geographies: &[Geography::State],
    period_label: "Year",
    series_note: "Percent of adults, with 95% confidence intervals.",
    measures: &[
        Measure { id: "mental_distress", label: "Frequent mental distress (14+ poor mental health days in 30)", code: None, unit: "percent" },
        Measure { id: "physical_distress", label: "Frequent physical distress (14+ poor physical health days in 30)", code: None, unit: "percent" },
        Measure { id: "activity_limitation", label: "Frequent activity limitation (14+ days in 30)", code: None, unit: "percent" },
        Measure { id: "depression", label: "Ever told they have a depressive disorder", code: None, unit: "percent" },
        Measure { id: "loneliness", label: "Feel lonely or socially isolated", code: None, unit: "percent" },
        Measure { id: "emotional_support", label: "Get the social and emotional support they need", code: None, unit: "percent" },
        Measure { id: "life_satisfaction", label: "Satisfied with their life", code: None, unit: "percent" },
    ],
    caveats: &[
        Caveat {
            title: "There's no national estimate",
            detail: "CDC publishes these indicators for states and territories only. Compare \
                     states with each other rather than averaging them.",
        },
        Caveat {
            title: "Values are survey estimates",
            detail: "Each value comes with a 95% confidence interval in `ci_low` and `ci_high`. \
                     When two states' intervals overlap, the difference may not be real.",
        },
        Caveat {
            title: "Some questions are optional",
            detail: "Loneliness, emotional support, and life satisfaction come from optional \
                     modules that only some states ask. A missing state means it wasn't asked.",
        },
        Caveat {
            title: "The loneliness question changed in 2023",
            detail: "It asked about feeling socially isolated in 2022 and about feeling lonely \
                     from 2023 on. Compare those years with care.",
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

impl Source for BrfssMentalHealth {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        META.check_measures(&query.measures)?;
        let mut clauses = vec![format!("demographics_type = {}", quote("Total"))];
        if let Some(geo) = &query.geo {
            clauses.push(format!("area_abbr = {}", quote(&geo.to_uppercase())));
        }
        if let Some(year) = query.since_year {
            clauses.push(format!("year >= {}", quote(&year.to_string())));
        }

        let rows: Vec<Row> =
            socrata::fetch_all(client, "data.cdc.gov", "5eh7-pjx8", &clauses.join(" AND "))?;
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
    Some(Observation {
        dataset: ID.into(),
        measure: measure.into(),
        geo: row.area_abbr,
        geo_name: Some(row.area),
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

    #[test]
    fn maps_questions_to_measures() {
        let row = Row {
            year: "2024".into(),
            question: "Now thinking about your mental health, which includes stress...".into(),
            area: "California".into(),
            area_abbr: "CA".into(),
            percent: Some("14.9".into()),
            low_confidence_interval: Some("14.5".into()),
            high_confidence_interval: Some("15.6".into()),
        };
        let obs = normalize(row).unwrap();
        assert_eq!(obs.measure, "mental_distress");
        assert_eq!(obs.value, Some(14.9));
        assert_eq!((obs.ci_low, obs.ci_high), (Some(14.5), Some(15.6)));
        assert_eq!(obs.period.to_string(), "2024");
    }
}
