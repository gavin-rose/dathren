//! Monitoring the Future: past-month substance use among 8th, 10th, and 12th graders,
//! as published in CDC's Health, United States.
//! <https://data.cdc.gov/d/mtgp-t7vw>

use serde::Deserialize;

use super::number;
use super::socrata::{self, quote};
use crate::{
    Caveat, Client, DatasetMeta, Geography, Measure, Observation, Period, Query, Result, Source,
};

pub struct MtfYouthSubstance;

const ID: &str = "mtf-youth-substance-use";

/// Measure id prefixes paired with the exact upstream `subgroup` strings.
const SUBSTANCES: &[(&str, &str)] = &[
    ("marijuana", "Marijuana"),
    ("alcohol", "Alcohol"),
    ("binge_drinking", "Binge drinking"),
    ("nicotine_vaping", "Nicotine vaping"),
    ("marijuana_vaping", "Marijuana vaping"),
    ("cigarettes", "Cigarettes"),
    ("cocaine", "Cocaine"),
    ("mdma", "MDMA (Ecstasy)"),
    ("inhalants", "Inhalants"),
];

/// Measure id suffixes paired with the exact upstream `group` strings.
const GRADES: &[(&str, &str)] = &[
    ("12th", "Substance use (12th Grade)"),
    ("10th", "Substance use (10th Grade)"),
    ("8th", "Substance use (8th Grade)"),
];

macro_rules! measures {
    ($($id:literal => $label:literal),* $(,)?) => {
        &[$(Measure { id: $id, label: $label, code: None, unit: "percent" }),*]
    };
}

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Youth Substance Use",
    publisher: "Monitoring the Future, via CDC Health, United States",
    summary: "Past-month use of marijuana, alcohol, vaping, cigarettes, and other drugs among \
              8th, 10th, and 12th graders nationally, with 12th grade going back to 1980.",
    source_url: "https://monitoringthefuture.org/",
    update_cadence: "Yearly",
    topics: &["substance use", "youth", "survey"],
    geographies: &[Geography::Nation],
    period_label: "Year",
    series_note: "Percent of students who used the substance in the past 30 days.",
    measures: measures![
        "marijuana_12th" => "Marijuana, 12th graders",
        "marijuana_10th" => "Marijuana, 10th graders",
        "marijuana_8th" => "Marijuana, 8th graders",
        "alcohol_12th" => "Alcohol, 12th graders",
        "alcohol_10th" => "Alcohol, 10th graders",
        "alcohol_8th" => "Alcohol, 8th graders",
        "binge_drinking_12th" => "Binge drinking, 12th graders",
        "binge_drinking_10th" => "Binge drinking, 10th graders",
        "binge_drinking_8th" => "Binge drinking, 8th graders",
        "nicotine_vaping_12th" => "Nicotine vaping, 12th graders",
        "nicotine_vaping_10th" => "Nicotine vaping, 10th graders",
        "nicotine_vaping_8th" => "Nicotine vaping, 8th graders",
        "marijuana_vaping_12th" => "Marijuana vaping, 12th graders",
        "marijuana_vaping_10th" => "Marijuana vaping, 10th graders",
        "marijuana_vaping_8th" => "Marijuana vaping, 8th graders",
        "cigarettes_12th" => "Cigarettes, 12th graders",
        "cigarettes_10th" => "Cigarettes, 10th graders",
        "cigarettes_8th" => "Cigarettes, 8th graders",
        "cocaine_12th" => "Cocaine, 12th graders",
        "cocaine_10th" => "Cocaine, 10th graders",
        "cocaine_8th" => "Cocaine, 8th graders",
        "mdma_12th" => "MDMA (ecstasy), 12th graders",
        "mdma_10th" => "MDMA (ecstasy), 10th graders",
        "mdma_8th" => "MDMA (ecstasy), 8th graders",
        "inhalants_12th" => "Inhalants, 12th graders",
        "inhalants_10th" => "Inhalants, 10th graders",
        "inhalants_8th" => "Inhalants, 8th graders",
    ],
    caveats: &[
        Caveat {
            title: "Values are past-month use",
            detail: "Each value is the percent of students who used the substance at least once \
                     in the 30 days before the survey, not regular or heavy use.",
        },
        Caveat {
            title: "Series start at different times",
            detail: "12th grade goes back to 1980, 8th and 10th grades start in 1991, and \
                     vaping questions start in 2017. Earlier years are empty, not zero.",
        },
        Caveat {
            title: "2020 had a smaller sample",
            detail: "The 2020 survey stopped when schools closed in March, so it covers fewer \
                     students than other years.",
        },
        Caveat {
            title: "National only, students in school",
            detail: "There are no state estimates, and students who have left school aren't \
                     surveyed. For state-level youth data, see the YRBSS dataset.",
        },
    ],
};

#[derive(Deserialize)]
struct Row {
    group: String,
    subgroup: String,
    time_period: String,
    estimate: Option<String>,
    estimate_lci: Option<String>,
    estimate_uci: Option<String>,
}

impl Source for MtfYouthSubstance {
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
        let mut clauses = vec![format!("classification = {}", quote("Total"))];
        if let Some(year) = query.since_year {
            clauses.push(format!("time_period >= {}", quote(&year.to_string())));
        }

        let rows: Vec<Row> =
            socrata::fetch_all(client, "data.cdc.gov", "mtgp-t7vw", &clauses.join(" AND "))?;
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
    let substance = SUBSTANCES.iter().find(|(_, s)| *s == row.subgroup)?.0;
    let grade = GRADES.iter().find(|(_, g)| *g == row.group)?.0;
    let value = number(row.estimate.as_deref());
    value?;
    Some(Observation {
        dataset: ID.into(),
        measure: format!("{substance}_{grade}"),
        geo: "US".into(),
        geo_name: Some("United States".into()),
        period: Period::year(row.time_period.parse().ok()?),
        value,
        adjusted_value: None,
        ci_low: number(row.estimate_lci.as_deref()),
        ci_high: number(row.estimate_uci.as_deref()),
        flags: Vec::new(),
        note: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(estimate: Option<&str>) -> Row {
        Row {
            group: "Substance use (12th Grade)".into(),
            subgroup: "Marijuana".into(),
            time_period: "2024".into(),
            estimate: estimate.map(Into::into),
            estimate_lci: None,
            estimate_uci: None,
        }
    }

    #[test]
    fn builds_measure_ids_from_substance_and_grade() {
        let obs = normalize(row(Some("16.2"))).unwrap();
        assert_eq!(
            (obs.measure.as_str(), obs.value),
            ("marijuana_12th", Some(16.2))
        );
        assert!(META.measure(&obs.measure).is_some());
    }

    #[test]
    fn years_before_a_series_started_are_skipped() {
        assert!(normalize(row(None)).is_none());
    }
}
