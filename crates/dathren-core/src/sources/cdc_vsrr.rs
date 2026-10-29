use serde::Deserialize;

use super::socrata::{self, quote};
use crate::{
    Caveat, Client, DatasetMeta, Error, Flag, Geography, Measure, Observation, Period, Query,
    Result, Source,
};

pub struct CdcVsrrOverdose;

const ID: &str = "cdc-vsrr-overdose";

const INDICATORS: &[(&str, &str)] = &[
    ("overdose_deaths", "Number of Drug Overdose Deaths"),
    ("opioids", "Opioids (T40.0-T40.4,T40.6)"),
    (
        "synthetic_opioids",
        "Synthetic opioids, excl. methadone (T40.4)",
    ),
    ("heroin", "Heroin (T40.1)"),
    (
        "natural_semisynthetic_opioids",
        "Natural & semi-synthetic opioids (T40.2)",
    ),
    ("methadone", "Methadone (T40.3)"),
    (
        "psychostimulants",
        "Psychostimulants with abuse potential (T43.6)",
    ),
    ("cocaine", "Cocaine (T40.5)"),
    ("all_deaths", "Number of Deaths"),
];

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Provisional Drug Overdose Death Counts",
    publisher: "CDC National Center for Health Statistics (VSRR)",
    summary: "Rolling 12-month counts of drug overdose deaths by state and drug class, \
              with CDC's predicted values that correct for delayed reporting.",
    source_url: "https://www.cdc.gov/nchs/nvss/vsrr/drug-overdose-data.htm",
    update_cadence: "Monthly",
    topics: &["substance use", "mortality", "opioids"],
    geographies: &[Geography::Nation, Geography::State],
    period_label: "12 months ending",
    series_note: "Rolling 12-month total, using CDC's predicted counts where available.",
    measures: &[
        Measure { id: "overdose_deaths", label: "All drug overdose deaths", code: None, unit: "deaths" },
        Measure { id: "opioids", label: "Opioids", code: Some("T40.0-T40.4, T40.6"), unit: "deaths" },
        Measure { id: "synthetic_opioids", label: "Synthetic opioids (mainly fentanyl), excl. methadone", code: Some("T40.4"), unit: "deaths" },
        Measure { id: "heroin", label: "Heroin", code: Some("T40.1"), unit: "deaths" },
        Measure { id: "natural_semisynthetic_opioids", label: "Natural & semi-synthetic opioids (e.g. oxycodone)", code: Some("T40.2"), unit: "deaths" },
        Measure { id: "methadone", label: "Methadone", code: Some("T40.3"), unit: "deaths" },
        Measure { id: "psychostimulants", label: "Psychostimulants (mainly methamphetamine)", code: Some("T43.6"), unit: "deaths" },
        Measure { id: "cocaine", label: "Cocaine", code: Some("T40.5"), unit: "deaths" },
        Measure { id: "all_deaths", label: "All deaths, any cause", code: None, unit: "deaths" },
    ],
    caveats: &[
        Caveat {
            title: "Values are rolling 12-month totals",
            detail: "Each period is the 12 months ending in that month, not a single month. \
                     Don't add consecutive periods together.",
        },
        Caveat {
            title: "Drug categories overlap",
            detail: "A death involving both fentanyl and cocaine counts in both categories. \
                     Drug-specific counts can't be summed to get the total.",
        },
        Caveat {
            title: "Recent months are underreported",
            detail: "Death investigations take months. `adjusted_value` is CDC's predicted count \
                     that corrects for this; `value` is what has been reported so far.",
        },
        Caveat {
            title: "Some state cells are suppressed",
            detail: "CDC withholds drug-specific counts for jurisdictions with low data quality. \
                     These are flagged `suppressed` with no value, not zero.",
        },
    ],
};

#[derive(Deserialize)]
struct Row {
    state: String,
    state_name: Option<String>,
    year: String,
    month: String,
    indicator: String,
    data_value: Option<String>,
    predicted_value: Option<String>,
    footnote: Option<String>,
    footnote_symbol: Option<String>,
}

impl Source for CdcVsrrOverdose {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        let mut clauses = vec![format!("period = {}", quote("12 month-ending"))];
        if let Some(geo) = &query.geo {
            clauses.push(format!("state = {}", quote(geo)));
        }
        if let Some(year) = query.since_year {
            clauses.push(format!("year >= {}", quote(&year.to_string())));
        }
        if !query.measures.is_empty() {
            let indicators = query
                .measures
                .iter()
                .map(|m| indicator_for(m).map(quote))
                .collect::<Result<Vec<_>>>()?;
            clauses.push(format!("indicator in ({})", indicators.join(", ")));
        }

        let rows: Vec<Row> =
            socrata::fetch_all(client, "data.cdc.gov", "xkb8-kh2a", &clauses.join(" AND "))?;
        let mut observations: Vec<_> = rows.into_iter().filter_map(normalize).collect();
        observations
            .sort_by(|a, b| (&a.measure, &a.geo, a.period).cmp(&(&b.measure, &b.geo, b.period)));
        Ok(observations)
    }
}

fn indicator_for(measure: &str) -> Result<&'static str> {
    INDICATORS
        .iter()
        .find(|(id, _)| *id == measure)
        .map(|(_, indicator)| *indicator)
        .ok_or_else(|| Error::UnknownMeasure {
            dataset: ID.into(),
            measure: measure.into(),
        })
}

/// Drops rows for indicators we don't model yet (e.g. "Percent with drugs specified").
fn normalize(row: Row) -> Option<Observation> {
    let measure = INDICATORS.iter().find(|(_, ind)| *ind == row.indicator)?.0;
    let year = row.year.parse().ok()?;
    let month = super::month_number(&row.month)?;
    let value = row.data_value.as_deref().and_then(|v| v.parse().ok());

    let mut flags = vec![Flag::Provisional];
    match row.footnote_symbol.as_deref() {
        Some(s) if s.contains("**") && value.is_none() => flags.push(Flag::Suppressed),
        Some(s) if s.contains('*') => flags.push(Flag::Underreported),
        _ if value.is_none() => flags.push(Flag::Suppressed),
        _ => {}
    }

    Some(Observation {
        dataset: ID.into(),
        measure: measure.into(),
        geo: row.state,
        geo_name: row.state_name,
        period: Period::month(year, month),
        value,
        adjusted_value: row.predicted_value.as_deref().and_then(|v| v.parse().ok()),
        ci_low: None,
        ci_high: None,
        flags,
        note: row.footnote,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(data_value: Option<&str>, symbol: Option<&str>) -> Row {
        Row {
            state: "AK".into(),
            state_name: Some("Alaska".into()),
            year: "2015".into(),
            month: "March".into(),
            indicator: "Cocaine (T40.5)".into(),
            data_value: data_value.map(Into::into),
            predicted_value: None,
            footnote: None,
            footnote_symbol: symbol.map(Into::into),
        }
    }

    #[test]
    fn missing_value_is_suppressed_not_zero() {
        let obs = normalize(row(None, Some("**"))).unwrap();
        assert_eq!(obs.value, None);
        assert!(obs.flags.contains(&Flag::Suppressed));
        assert_eq!(obs.period.to_string(), "2015-03");
    }

    #[test]
    fn single_star_is_underreported() {
        let obs = normalize(row(Some("12"), Some("*"))).unwrap();
        assert_eq!(obs.value, Some(12.0));
        assert!(obs.flags.contains(&Flag::Underreported));
        assert!(!obs.flags.contains(&Flag::Suppressed));
    }

    #[test]
    fn best_prefers_adjusted_value() {
        let mut r = row(Some("65351"), Some("*"));
        r.predicted_value = Some("66478".into());
        assert_eq!(normalize(r).unwrap().best(), Some(66478.0));
    }

    #[test]
    fn unmodeled_indicators_are_dropped() {
        let mut r = row(Some("90"), None);
        r.indicator = "Percent with drugs specified".into();
        assert!(normalize(r).is_none());
    }
}
