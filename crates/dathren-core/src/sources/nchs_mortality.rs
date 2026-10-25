//! NCHS VSRR quarterly provisional estimates for selected indicators of mortality.
//! <https://data.cdc.gov/d/489q-934x>

use serde_json::{Map, Value};

use super::number;
use super::socrata::{self, quote};
use crate::geo::STATES;
use crate::{
    Caveat, Client, DatasetMeta, Flag, Geography, Measure, Observation, Period, Query, Result,
    Source,
};

pub struct NchsMortality;

const ID: &str = "nchs-mortality-rates";

/// Measure ids paired with the exact upstream `cause_of_death` strings.
const CAUSES: &[(&str, &str)] = &[
    ("all_causes", "All causes"),
    ("alzheimer", "Alzheimer disease"),
    ("cancer", "Cancer"),
    ("liver_disease", "Chronic liver disease and cirrhosis"),
    ("lower_respiratory", "Chronic lower respiratory diseases"),
    ("covid_19", "COVID-19"),
    ("diabetes", "Diabetes"),
    ("drug_overdose", "Drug overdose"),
    ("falls_65_plus", "Falls, ages 65 and over"),
    ("firearm", "Firearm-related injury"),
    ("heart_disease", "Heart disease"),
    ("hiv", "HIV disease"),
    ("homicide", "Homicide"),
    ("hypertension", "Hypertension"),
    ("flu_pneumonia", "Influenza and pneumonia"),
    ("kidney_disease", "Kidney disease"),
    ("parkinson", "Parkinson disease"),
    ("pneumonitis", "Pneumonitis due to solids and liquids"),
    ("stroke", "Stroke"),
    ("suicide", "Suicide"),
    ("unintentional_injury", "Unintentional injuries"),
];

const UNIT: &str = "deaths per 100,000";

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Provisional Death Rates by Cause",
    publisher: "CDC National Center for Health Statistics (VSRR)",
    summary: "Age-adjusted death rates for the leading causes of death plus overdose, firearms, \
              homicide, and suicide, nationally and by state, updated each quarter.",
    source_url: "https://www.cdc.gov/nchs/nvss/vsrr/mortality-dashboard.htm",
    update_cadence: "Quarterly",
    topics: &["mortality", "injury", "chronic disease"],
    geographies: &[Geography::Nation, Geography::State],
    period_label: "12 months ending",
    series_note: "Age-adjusted rate per 100,000 for the 12 months ending each quarter.",
    measures: &[
        Measure { id: "suicide", label: "Suicide", code: Some("*U03, X60-X84, Y87.0"), unit: UNIT },
        Measure { id: "all_causes", label: "All causes", code: None, unit: UNIT },
        Measure { id: "alzheimer", label: "Alzheimer disease", code: Some("G30"), unit: UNIT },
        Measure { id: "cancer", label: "Cancer", code: Some("C00-C97"), unit: UNIT },
        Measure { id: "liver_disease", label: "Chronic liver disease and cirrhosis", code: Some("K70, K73-K74"), unit: UNIT },
        Measure { id: "lower_respiratory", label: "Chronic lower respiratory diseases", code: Some("J40-J47"), unit: UNIT },
        Measure { id: "covid_19", label: "COVID-19", code: Some("U07.1"), unit: UNIT },
        Measure { id: "diabetes", label: "Diabetes", code: Some("E10-E14"), unit: UNIT },
        Measure { id: "drug_overdose", label: "Drug overdose", code: Some("X40-X44, X60-X64, X85, Y10-Y14"), unit: UNIT },
        Measure { id: "falls_65_plus", label: "Falls, ages 65 and over", code: Some("W00-W19"), unit: UNIT },
        Measure { id: "firearm", label: "Firearm-related injury", code: Some("*U01.4, W32-W34, X72-X74, X93-X95, Y22-Y24, Y35.0"), unit: UNIT },
        Measure { id: "heart_disease", label: "Heart disease", code: Some("I00-I09, I11, I13, I20-I51"), unit: UNIT },
        Measure { id: "hiv", label: "HIV disease", code: Some("B20-B24"), unit: UNIT },
        Measure { id: "homicide", label: "Homicide", code: Some("*U01-*U02, X85-Y09, Y87.1"), unit: UNIT },
        Measure { id: "hypertension", label: "Hypertension", code: Some("I10, I12, I15"), unit: UNIT },
        Measure { id: "flu_pneumonia", label: "Influenza and pneumonia", code: Some("J09-J18"), unit: UNIT },
        Measure { id: "kidney_disease", label: "Kidney disease", code: Some("N00-N07, N17-N19, N25-N27"), unit: UNIT },
        Measure { id: "parkinson", label: "Parkinson disease", code: Some("G20-G21"), unit: UNIT },
        Measure { id: "pneumonitis", label: "Pneumonitis due to solids and liquids", code: Some("J69"), unit: UNIT },
        Measure { id: "stroke", label: "Stroke", code: Some("I60-I69"), unit: UNIT },
        Measure { id: "unintentional_injury", label: "Unintentional injuries", code: Some("V01-X59, Y85-Y86"), unit: UNIT },
    ],
    caveats: &[
        Caveat {
            title: "Rates are rolling 12-month figures",
            detail: "A period like `2025-Q3` covers the 12 months ending with that quarter. \
                     Consecutive periods overlap, so don't add or average them.",
        },
        Caveat {
            title: "Rates are age-adjusted",
            detail: "Rates are standardized to the 2000 US population so states with older or \
                     younger residents can be compared fairly. They aren't death counts.",
        },
        Caveat {
            title: "Causes overlap",
            detail: "Drug overdose and firearm deaths include suicides, homicides, and \
                     unintentional injuries. Don't add causes together.",
        },
        Caveat {
            title: "Recent quarters are provisional",
            detail: "Deaths still under investigation, especially overdoses and injuries, can \
                     shift rates when final data is released.",
        },
        Caveat {
            title: "Some state rates are missing",
            detail: "CDC leaves out rates that are unreliable because of small numbers. These are \
                     flagged `suppressed` with no value, not zero.",
        },
    ],
};

impl Source for NchsMortality {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        META.check_measures(&query.measures)?;
        let mut clauses = vec![
            format!("time_period = {}", quote("12 months ending with quarter")),
            format!("rate_type = {}", quote("Age-adjusted")),
        ];
        if let Some(year) = query.since_year {
            clauses.push(format!("year_and_quarter >= {}", quote(&year.to_string())));
        }
        if !query.measures.is_empty() {
            let causes: Vec<String> = CAUSES
                .iter()
                .filter(|(id, _)| query.measures.iter().any(|m| m == id))
                .map(|(_, cause)| quote(cause))
                .collect();
            clauses.push(format!("cause_of_death in ({})", causes.join(", ")));
        }

        let rows: Vec<Map<String, Value>> =
            socrata::fetch_all(client, "data.cdc.gov", "489q-934x", &clauses.join(" AND "))?;
        let wanted_geo = query.geo.as_deref().map(str::to_uppercase);
        let mut observations: Vec<Observation> = rows
            .iter()
            .flat_map(normalize)
            .filter(|o| wanted_geo.as_deref().is_none_or(|g| o.geo == g))
            .collect();
        observations
            .sort_by(|a, b| (&a.measure, &a.geo, a.period).cmp(&(&b.measure, &b.geo, b.period)));
        Ok(observations)
    }
}

/// Unpivots one wide row, with a column per state, into an observation per geography.
fn normalize(row: &Map<String, Value>) -> Vec<Observation> {
    let text = |key: &str| row.get(key).and_then(Value::as_str);
    let Some(measure) = text("cause_of_death")
        .and_then(|c| CAUSES.iter().find(|(_, cause)| *cause == c))
        .map(|(id, _)| *id)
    else {
        return Vec::new();
    };
    let Some(period) = text("year_and_quarter").and_then(parse_quarter) else {
        return Vec::new();
    };
    if text("rate_overall").is_none() {
        return Vec::new();
    }

    let national = (
        "US".to_string(),
        "United States".to_string(),
        "rate_overall".to_string(),
    );
    let states = STATES.iter().map(|s| {
        let column = format!("rate_{}", s.name.to_lowercase().replace([' ', '.'], "_"));
        (s.abbr.to_string(), s.name.to_string(), column)
    });
    std::iter::once(national)
        .chain(states)
        .filter(|(abbr, ..)| abbr != "GU" && abbr != "PR" && abbr != "VI")
        .map(|(geo, name, column)| {
            let value = number(text(&column));
            let mut flags = vec![Flag::Provisional];
            if value.is_none() {
                flags.push(Flag::Suppressed);
            }
            Observation {
                dataset: ID.into(),
                measure: measure.into(),
                geo,
                geo_name: Some(name),
                period,
                value,
                adjusted_value: None,
                ci_low: None,
                ci_high: None,
                flags,
                note: None,
            }
        })
        .collect()
}

/// Parses `2025 Q3`.
fn parse_quarter(raw: &str) -> Option<Period> {
    let (year, quarter) = raw.split_once(" Q")?;
    Some(Period::quarter(year.parse().ok()?, quarter.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unpivots_state_columns() {
        let row = json!({
            "year_and_quarter": "2025 Q3",
            "cause_of_death": "Suicide",
            "rate_overall": "14.1",
            "rate_alaska": "30.2",
            "rate_district_of_columbia": "6.4"
        });
        let obs = normalize(row.as_object().unwrap());
        let get = |geo: &str| obs.iter().find(|o| o.geo == geo).unwrap();
        assert_eq!(get("US").value, Some(14.1));
        assert_eq!(get("AK").value, Some(30.2));
        assert_eq!(get("DC").value, Some(6.4));
        assert_eq!(get("US").period.to_string(), "2025-Q3");
        assert!(get("WY").flags.contains(&Flag::Suppressed));
        assert!(obs.iter().all(|o| o.geo != "PR"));
    }

    #[test]
    fn skips_quarters_not_yet_released() {
        let row = json!({ "year_and_quarter": "2025 Q3", "cause_of_death": "Suicide" });
        assert!(normalize(row.as_object().unwrap()).is_empty());
    }
}
