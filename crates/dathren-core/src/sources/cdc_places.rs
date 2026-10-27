//! CDC PLACES county-level estimates, 2025 release.
//! <https://data.cdc.gov/d/swc5-untb>

use serde::Deserialize;

use super::number;
use super::socrata::{self, quote};
use crate::{
    Caveat, Client, DatasetMeta, Flag, Geography, Measure, Observation, Period, Query, Result,
    Source,
};

pub struct CdcPlaces;

const ID: &str = "cdc-places-county";

macro_rules! measures {
    ($($id:literal => $label:literal),* $(,)?) => {
        &[$(Measure { id: $id, label: $label, code: None, unit: "percent" }),*]
    };
}

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "PLACES: Local Health Estimates by County",
    publisher: "CDC Division of Population Health (PLACES)",
    summary: "Model-based estimates of 40 health outcomes, behaviors, disabilities, and social \
              needs for every US county.",
    source_url: "https://www.cdc.gov/places/",
    update_cadence: "Yearly",
    topics: &["chronic disease", "mental health", "local", "social needs"],
    geographies: &[Geography::County],
    period_label: "Year",
    series_note: "Age-adjusted percent of adults, with 95% confidence intervals.",
    measures: measures![
        "mhlth" => "Frequent mental distress",
        "depression" => "Depression",
        "loneliness" => "Loneliness",
        "emotionspt" => "Lack of social and emotional support",
        "phlth" => "Frequent physical distress",
        "ghlth" => "Fair or poor self-rated health",
        "arthritis" => "Arthritis",
        "bphigh" => "High blood pressure",
        "cancer" => "Cancer (non-skin) or melanoma",
        "casthma" => "Current asthma",
        "chd" => "Coronary heart disease",
        "copd" => "COPD",
        "diabetes" => "Diagnosed diabetes",
        "highchol" => "High cholesterol among adults ever screened",
        "obesity" => "Obesity",
        "stroke" => "Stroke",
        "teethlost" => "All teeth lost, ages 65 and over",
        "binge" => "Binge drinking",
        "csmoking" => "Current cigarette smoking",
        "lpa" => "No leisure-time physical activity",
        "sleep" => "Short sleep duration",
        "access2" => "No health insurance, ages 18 to 64",
        "bpmed" => "Taking blood pressure medication, among those with high blood pressure",
        "checkup" => "Routine checkup in the past year",
        "cholscreen" => "Cholesterol screening",
        "colon_screen" => "Colorectal cancer screening, ages 45 to 75",
        "dental" => "Dental visit in the past year",
        "mammouse" => "Mammography, women ages 50 to 74",
        "cognition" => "Cognitive disability",
        "disability" => "Any disability",
        "hearing" => "Hearing disability",
        "indeplive" => "Independent living disability",
        "mobility" => "Mobility disability",
        "selfcare" => "Self-care disability",
        "vision" => "Vision disability",
        "foodinsecu" => "Food insecurity in the past year",
        "foodstamp" => "Received food stamps in the past year",
        "housinsecu" => "Housing insecurity in the past year",
        "lacktrpt" => "Lack of reliable transportation in the past year",
        "shututility" => "Threat of utility shut-off in the past year",
    ],
    caveats: &[
        Caveat {
            title: "These are model-based estimates",
            detail: "CDC estimates each county from state survey data with a statistical model. \
                     They're good for comparing places, not for tracking a county year to year.",
        },
        Caveat {
            title: "Values are age-adjusted",
            detail: "Dathren uses the age-adjusted estimates so counties can be compared. CDC \
                     also publishes crude estimates, which suit local planning better.",
        },
        Caveat {
            title: "Measures come from different years",
            detail: "One release mixes survey years, for example 2022 and 2023. Check `period` \
                     before comparing two measures.",
        },
        Caveat {
            title: "Kentucky and Pennsylvania are mostly missing",
            detail: "This release has only 5 measures for their counties (screenings, dental \
                     visits, sleep, and tooth loss), all from 2022. The rest aren't published.",
        },
        Caveat {
            title: "Some measures cover narrower groups",
            detail: "Health insurance covers ages 18 to 64, mammography women 50 to 74, and \
                     tooth loss ages 65 and over. The measure labels say so.",
        },
    ],
};

#[derive(Deserialize)]
struct Row {
    year: String,
    stateabbr: String,
    locationname: Option<String>,
    locationid: String,
    measureid: String,
    data_value: Option<String>,
    data_value_footnote: Option<String>,
    low_confidence_limit: Option<String>,
    high_confidence_limit: Option<String>,
}

impl Source for CdcPlaces {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        META.check_measures(&query.measures)?;
        let mut clauses = vec![format!("datavaluetypeid = {}", quote("AgeAdjPrv"))];
        match query.geo.as_deref() {
            Some(geo) if geo.len() == 2 => {
                clauses.push(format!("stateabbr = {}", quote(&geo.to_uppercase())))
            }
            Some(geo) => clauses.push(format!("locationid = {}", quote(geo))),
            None => {}
        }
        if let Some(year) = query.since_year {
            clauses.push(format!("year >= {}", quote(&year.to_string())));
        }
        if !query.measures.is_empty() {
            let ids: Vec<String> = query
                .measures
                .iter()
                .map(|m| quote(&m.to_uppercase()))
                .collect();
            clauses.push(format!("measureid in ({})", ids.join(", ")));
        }

        let rows: Vec<Row> =
            socrata::fetch_all(client, "data.cdc.gov", "swc5-untb", &clauses.join(" AND "))?;
        let mut observations: Vec<Observation> = rows.into_iter().filter_map(normalize).collect();
        observations
            .sort_by(|a, b| (&a.measure, &a.geo, a.period).cmp(&(&b.measure, &b.geo, b.period)));
        Ok(observations)
    }
}

fn normalize(row: Row) -> Option<Observation> {
    let measure = row.measureid.to_lowercase();
    META.measure(&measure)?;
    let value = number(row.data_value.as_deref());
    let flags = if value.is_none() {
        vec![Flag::Suppressed]
    } else {
        Vec::new()
    };
    let (geo, geo_name) = match row.locationname {
        _ if row.stateabbr == "US" => ("US".to_string(), "United States".to_string()),
        Some(name) => (row.locationid, format!("{name}, {}", row.stateabbr)),
        None => (row.locationid.clone(), row.locationid),
    };
    Some(Observation {
        dataset: ID.into(),
        measure,
        geo,
        geo_name: Some(geo_name),
        period: Period::year(row.year.parse().ok()?),
        value,
        adjusted_value: None,
        ci_low: number(row.low_confidence_limit.as_deref()),
        ci_high: number(row.high_confidence_limit.as_deref()),
        flags,
        note: row.data_value_footnote,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_rows() {
        let row = Row {
            year: "2023".into(),
            stateabbr: "AR".into(),
            locationname: Some("Drew".into()),
            locationid: "05043".into(),
            measureid: "MHLTH".into(),
            data_value: Some("19.8".into()),
            data_value_footnote: None,
            low_confidence_limit: Some("17.9".into()),
            high_confidence_limit: Some("21.7".into()),
        };
        let obs = normalize(row).unwrap();
        assert_eq!(obs.measure, "mhlth");
        assert_eq!(obs.geo_name.as_deref(), Some("Drew, AR"));
        assert_eq!(obs.ci_high, Some(21.7));
    }

    #[test]
    fn national_rows_become_us() {
        let row = Row {
            year: "2023".into(),
            stateabbr: "US".into(),
            locationname: None,
            locationid: "59".into(),
            measureid: "MHLTH".into(),
            data_value: Some("16.1".into()),
            data_value_footnote: None,
            low_confidence_limit: None,
            high_confidence_limit: None,
        };
        let obs = normalize(row).unwrap();
        assert_eq!(
            (obs.geo.as_str(), obs.geo_name.as_deref()),
            ("US", Some("United States"))
        );
    }
}
