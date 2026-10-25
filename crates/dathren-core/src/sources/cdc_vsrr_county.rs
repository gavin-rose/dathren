//! CDC VSRR provisional county-level drug overdose death counts.
//! <https://data.cdc.gov/d/gb4e-yj24>

use serde::Deserialize;

use super::number;
use super::socrata::{self, quote};
use crate::geo::county_fips;
use crate::{
    Caveat, Client, DatasetMeta, Flag, Geography, Measure, Observation, Period, Query, Result,
    Source,
};

pub struct CdcVsrrCounty;

const ID: &str = "cdc-vsrr-county-overdose";

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Provisional County-Level Drug Overdose Deaths",
    publisher: "CDC National Center for Health Statistics (VSRR)",
    summary: "Rolling 12-month counts of drug overdose deaths for every US county, by the \
              county where the person lived.",
    source_url: "https://www.cdc.gov/nchs/nvss/vsrr/prov-county-drug-overdose.htm",
    update_cadence: "Quarterly",
    topics: &["substance use", "mortality", "local"],
    geographies: &[Geography::County],
    period_label: "12 months ending",
    series_note: "Overdose deaths in the 12 months ending with the latest month.",
    measures: &[Measure {
        id: "overdose_deaths",
        label: "All drug overdose deaths",
        code: Some("X40-X44, X60-X64, X85, Y10-Y14"),
        unit: "deaths",
    }],
    caveats: &[
        Caveat {
            title: "Counts of 1 to 9 are suppressed",
            detail: "CDC hides small counts to protect privacy. They're flagged `suppressed` with \
                     no value. A `0` is a real zero.",
        },
        Caveat {
            title: "Values are rolling 12-month totals",
            detail: "Each period is the 12 months ending in that month. Don't add consecutive \
                     periods together.",
        },
        Caveat {
            title: "Counts are provisional and lag about six months",
            detail: "Overdose deaths take months to investigate, so recent counts are likely to \
                     rise. `note` carries CDC's completeness warnings where they apply.",
        },
        Caveat {
            title: "Small counties swing a lot",
            detail: "A few deaths can change a small county's total sharply. Compare counties \
                     using rates, not raw counts, when populations differ.",
        },
    ],
};

#[derive(Deserialize)]
struct Row {
    year: String,
    month: String,
    st_abbrev: String,
    countyname: String,
    fips: String,
    provisional_drug_overdose: Option<String>,
    footnote: Option<String>,
    historicaldatacompletenessnote: Option<String>,
}

impl Source for CdcVsrrCounty {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        META.check_measures(&query.measures)?;
        let mut clauses = vec!["fips IS NOT NULL".to_string()];
        match query.geo.as_deref() {
            Some(geo) if geo.len() == 2 => {
                clauses.push(format!("st_abbrev = {}", quote(&geo.to_uppercase())))
            }
            Some(geo) => clauses.push(format!("fips = {}", quote(geo.trim_start_matches('0')))),
            None => {}
        }
        if let Some(year) = query.since_year {
            clauses.push(format!("year >= {}", quote(&year.to_string())));
        }

        let rows: Vec<Row> =
            socrata::fetch_all(client, "data.cdc.gov", "gb4e-yj24", &clauses.join(" AND "))?;
        let mut observations: Vec<Observation> = rows.into_iter().filter_map(normalize).collect();
        observations.sort_by(|a, b| (&a.geo, a.period).cmp(&(&b.geo, b.period)));
        Ok(observations)
    }
}

fn normalize(row: Row) -> Option<Observation> {
    let value = number(row.provisional_drug_overdose.as_deref());
    let mut flags = vec![Flag::Provisional];
    if value.is_none() {
        flags.push(Flag::Suppressed);
    }
    let completeness = row
        .historicaldatacompletenessnote
        .filter(|n| !n.trim().is_empty());
    if completeness.is_some() {
        flags.push(Flag::Underreported);
    }
    Some(Observation {
        dataset: ID.into(),
        measure: "overdose_deaths".into(),
        geo: county_fips(&row.fips),
        geo_name: Some(format!("{}, {}", row.countyname, row.st_abbrev)),
        period: Period::month(row.year.parse().ok()?, row.month.parse().ok()?),
        value,
        adjusted_value: None,
        ci_low: None,
        ci_high: None,
        flags,
        note: completeness.or(row.footnote),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(count: Option<&str>) -> Row {
        Row {
            year: "2026".into(),
            month: "3".into(),
            st_abbrev: "AL".into(),
            countyname: "Autauga".into(),
            fips: "1001".into(),
            provisional_drug_overdose: count.map(Into::into),
            footnote: None,
            historicaldatacompletenessnote: None,
        }
    }

    #[test]
    fn pads_fips_and_keeps_real_zeros() {
        let obs = normalize(row(Some("0"))).unwrap();
        assert_eq!(obs.geo, "01001");
        assert_eq!(obs.geo_name.as_deref(), Some("Autauga, AL"));
        assert_eq!(obs.value, Some(0.0));
        assert!(!obs.flags.contains(&Flag::Suppressed));
    }

    #[test]
    fn missing_counts_are_suppressed() {
        let obs = normalize(row(None)).unwrap();
        assert_eq!(obs.value, None);
        assert!(obs.flags.contains(&Flag::Suppressed));
    }
}
