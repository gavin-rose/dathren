//! CDC Mapping Injury, Overdose, and Violence: county death rates.
//! <https://data.cdc.gov/d/psx4-wq38>

use serde::Deserialize;

use super::number;
use super::socrata::{self, quote};
use crate::geo::{self, county_fips};
use crate::{
    Caveat, Client, DatasetMeta, Flag, Geography, Measure, Observation, Period, Query, Result,
    Source,
};

pub struct CdcInjuryCounty;

const ID: &str = "cdc-injury-county";
const UNIT: &str = "deaths per 100,000";

/// Measure ids paired with the exact upstream `intent` strings.
const INTENTS: &[(&str, &str)] = &[
    ("suicide", "All_Suicide"),
    ("drug_overdose", "Drug_OD"),
    ("homicide", "All_Homicide"),
    ("firearm", "FA_Deaths"),
    ("firearm_suicide", "FA_Suicide"),
    ("firearm_homicide", "FA_Homicide"),
];

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Injury and Violence Death Rates by County",
    publisher: "CDC Mapping Injury, Overdose, and Violence",
    summary: "County death rates for suicide, drug overdose, homicide, and firearm injuries, \
              yearly since 2019 and for the most recent 12 months.",
    source_url: "https://www.cdc.gov/injury/",
    update_cadence: "Monthly",
    topics: &["injury", "suicide", "firearms", "substance use", "local"],
    geographies: &[Geography::County],
    period_label: "Period",
    series_note: "Deaths per 100,000 residents in the most recent 12 months.",
    measures: &[
        Measure { id: "suicide", label: "Suicide", code: None, unit: UNIT },
        Measure { id: "drug_overdose", label: "Drug overdose", code: None, unit: UNIT },
        Measure { id: "homicide", label: "Homicide", code: None, unit: UNIT },
        Measure { id: "firearm", label: "Firearm deaths, all intents", code: None, unit: UNIT },
        Measure { id: "firearm_suicide", label: "Firearm suicide", code: None, unit: UNIT },
        Measure { id: "firearm_homicide", label: "Firearm homicide", code: None, unit: UNIT },
    ],
    caveats: &[
        Caveat {
            title: "Small counts are modeled",
            detail: "When a county had 1 to 9 deaths, CDC estimates its rate with a statistical \
                     model that borrows from neighboring areas. These are flagged `modeled` and \
                     come with a 95% confidence interval.",
        },
        Caveat {
            title: "Some small counts have no rate",
            detail: "Where CDC withholds even a modeled rate, the value is empty and flagged \
                     `suppressed`, not zero.",
        },
        Caveat {
            title: "Periods mix years and recent months",
            detail: "A period like `2024` is that calendar year. A period like `2026-05` is the 12 \
                     months ending in that month, which is the newest data.",
        },
        Caveat {
            title: "Firearm deaths overlap other causes",
            detail: "Firearm suicides are also suicides, and firearm homicides are also homicides. \
                     Don't add measures together.",
        },
        Caveat {
            title: "Rates are by county of residence",
            detail: "Deaths count where the person lived, not where they died. Recent months are \
                     provisional and may change.",
        },
    ],
};

#[derive(Deserialize)]
struct Row {
    geoid: String,
    name: String,
    st_geoid: Option<String>,
    intent: String,
    period: String,
    rate: Option<String>,
    rate_m: Option<String>,
    rate_m_ci: Option<String>,
    ttm_date_range: Option<String>,
}

impl Source for CdcInjuryCounty {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        META.check_measures(&query.measures)?;
        let mut clauses = Vec::new();
        match query.geo.as_deref() {
            Some(geo) if geo.len() == 2 => {
                let state = geo::state(geo).map_or("", |s| s.fips);
                clauses.push(format!("st_geoid = {}", quote(state)));
            }
            Some(geo) => clauses.push(format!("geoid = {}", quote(geo))),
            None => {}
        }
        if let Some(year) = query.since_year {
            clauses.push(format!(
                "(period >= {} OR period = {})",
                quote(&year.to_string()),
                quote("TTM")
            ));
        }
        if !query.measures.is_empty() {
            let intents: Vec<String> = INTENTS
                .iter()
                .filter(|(id, _)| query.measures.iter().any(|m| m == id))
                .map(|(_, intent)| quote(intent))
                .collect();
            clauses.push(format!("intent in ({})", intents.join(", ")));
        }
        let filter = if clauses.is_empty() {
            "1 = 1".to_string()
        } else {
            clauses.join(" AND ")
        };

        let rows: Vec<Row> = socrata::fetch_all(client, "data.cdc.gov", "psx4-wq38", &filter)?;
        let mut observations: Vec<Observation> = rows.into_iter().filter_map(normalize).collect();
        observations
            .sort_by(|a, b| (&a.measure, &a.geo, a.period).cmp(&(&b.measure, &b.geo, b.period)));
        Ok(observations)
    }
}

fn normalize(row: Row) -> Option<Observation> {
    let measure = INTENTS.iter().find(|(_, intent)| *intent == row.intent)?.0;
    let period = match row.period.as_str() {
        "TTM" => ttm_end(row.ttm_date_range.as_deref()?)?,
        year => Period::year(year.parse().ok()?),
    };
    let value = number(row.rate.as_deref()).filter(|rate| *rate >= 0.0);
    let modeled = row.rate_m.as_deref().and_then(|m| m.parse::<f64>().ok()) == Some(1.0);
    let (ci_low, ci_high) = match row.rate_m_ci.as_deref().and_then(|ci| ci.split_once('-')) {
        Some((lo, hi)) if modeled && value.is_some() => (number(Some(lo)), number(Some(hi))),
        _ => (None, None),
    };
    let mut flags = Vec::new();
    if value.is_none() {
        flags.push(Flag::Suppressed);
    } else if modeled {
        flags.push(Flag::Modeled);
    }
    let state = row
        .st_geoid
        .as_deref()
        .and_then(|fips| geo::STATES.iter().find(|s| s.fips == format!("{fips:0>2}")))
        .map_or(String::new(), |s| format!(", {}", s.abbr));
    Some(Observation {
        dataset: ID.into(),
        measure: measure.into(),
        geo: county_fips(&row.geoid),
        geo_name: Some(format!("{}{state}", row.name)),
        period,
        value,
        adjusted_value: None,
        ci_low,
        ci_high,
        flags,
        note: None,
    })
}

/// Parses the end of `June, 2025 to May, 2026`.
fn ttm_end(range: &str) -> Option<Period> {
    let (_, end) = range.split_once(" to ")?;
    let (month, year) = end.split_once(", ")?;
    Some(Period::month(
        year.trim().parse().ok()?,
        super::month_number(month.trim())?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(period: &str, rate_m: &str, ci: Option<&str>) -> Row {
        Row {
            geoid: "54059".into(),
            name: "Mingo County".into(),
            st_geoid: Some("54".into()),
            intent: "All_Suicide".into(),
            period: period.into(),
            rate: Some("17.6".into()),
            rate_m: Some(rate_m.into()),
            rate_m_ci: ci.map(Into::into),
            ttm_date_range: Some("June, 2025 to May, 2026".into()),
        }
    }

    #[test]
    fn parses_trailing_twelve_months_and_models() {
        let obs = normalize(row("TTM", "1.000000000000000", Some("12.2-24.6"))).unwrap();
        assert_eq!(obs.period.to_string(), "2026-05");
        assert_eq!(obs.geo_name.as_deref(), Some("Mingo County, WV"));
        assert!(obs.flags.contains(&Flag::Modeled));
        assert_eq!((obs.ci_low, obs.ci_high), (Some(12.2), Some(24.6)));
    }

    #[test]
    fn negative_placeholder_rates_are_suppressed() {
        let mut r = row("TTM", "1.000000000000000", Some("-999"));
        r.rate = Some("-999.0000000000000".into());
        let obs = normalize(r).unwrap();
        assert_eq!((obs.value, obs.ci_low, obs.ci_high), (None, None, None));
        assert_eq!(obs.flags, vec![Flag::Suppressed]);
    }

    #[test]
    fn direct_rates_have_no_interval() {
        let obs = normalize(row("2023", "0.000000000000000", None)).unwrap();
        assert_eq!(obs.period.to_string(), "2023");
        assert!(obs.flags.is_empty());
        assert_eq!(obs.ci_low, None);
    }
}
