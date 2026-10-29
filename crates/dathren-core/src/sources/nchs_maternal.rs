//! NCHS VSRR provisional maternal death counts and rates.
//! <https://data.cdc.gov/d/e2d5-ggg7>

use serde::Deserialize;

use super::number;
use super::socrata::{self, quote};
use crate::{
    Caveat, Client, DatasetMeta, Flag, Geography, Measure, Observation, Period, Query, Result,
    Source,
};

pub struct NchsMaternal;

const ID: &str = "nchs-maternal-mortality";

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Provisional Maternal Mortality",
    publisher: "CDC National Center for Health Statistics (VSRR)",
    summary: "National maternal death counts and rates per 100,000 live births, as rolling \
              12-month figures updated through the most recent provisional data.",
    source_url: "https://www.cdc.gov/nchs/maternal-mortality/",
    update_cadence: "Quarterly",
    topics: &["maternal health", "mortality"],
    geographies: &[Geography::Nation],
    period_label: "12 months ending",
    series_note: "Maternal deaths per 100,000 live births in the 12 months ending each month.",
    measures: &[
        Measure { id: "maternal_mortality_rate", label: "Maternal mortality rate", code: Some("A34, O00-O95, O98-O99"), unit: "deaths per 100,000 live births" },
        Measure { id: "maternal_deaths", label: "Maternal deaths", code: Some("A34, O00-O95, O98-O99"), unit: "deaths" },
    ],
    caveats: &[
        Caveat {
            title: "What counts as a maternal death",
            detail: "A death during pregnancy or within 42 days after it, from causes related to \
                     or made worse by the pregnancy. Accidents and unrelated causes don't count.",
        },
        Caveat {
            title: "Values are rolling 12-month figures",
            detail: "Each period is the 12 months ending in that month. Don't add consecutive \
                     periods together.",
        },
        Caveat {
            title: "Recent months are provisional",
            detail: "Both deaths and births are still being reported for recent months, so the \
                     rate can change when final data is released.",
        },
        Caveat {
            title: "Breakdowns aren't included yet",
            detail: "CDC also publishes rates by age and by race and Hispanic origin. Dathren \
                     includes the national total only for now.",
        },
    ],
};

#[derive(Deserialize)]
struct Row {
    year_of_death: String,
    month_of_death: String,
    maternal_deaths: Option<String>,
    maternal_mortality_rate: Option<String>,
    footnote: Option<String>,
}

impl Source for NchsMaternal {
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
            format!("`group` = {}", quote("Total")),
            format!("time_period = {}", quote("12 month-ending")),
        ];
        if let Some(year) = query.since_year {
            clauses.push(format!("year_of_death >= {}", quote(&year.to_string())));
        }

        let rows: Vec<Row> =
            socrata::fetch_all(client, "data.cdc.gov", "e2d5-ggg7", &clauses.join(" AND "))?;
        let mut observations: Vec<Observation> = rows
            .into_iter()
            .flat_map(normalize)
            .filter(|o| query.measures.is_empty() || query.measures.contains(&o.measure))
            .collect();
        observations.sort_by(|a, b| (&a.measure, a.period).cmp(&(&b.measure, b.period)));
        Ok(observations)
    }
}

fn normalize(row: Row) -> Vec<Observation> {
    let (Ok(year), Ok(month)) = (row.year_of_death.parse(), row.month_of_death.parse()) else {
        return Vec::new();
    };
    [
        ("maternal_mortality_rate", &row.maternal_mortality_rate),
        ("maternal_deaths", &row.maternal_deaths),
    ]
    .into_iter()
    .map(|(measure, raw)| {
        let value = number(raw.as_deref());
        let mut flags = vec![Flag::Provisional];
        if value.is_none() {
            flags.push(Flag::Suppressed);
        }
        Observation {
            dataset: ID.into(),
            measure: measure.into(),
            geo: "US".into(),
            geo_name: Some("United States".into()),
            period: Period::month(year, month),
            value,
            adjusted_value: None,
            ci_low: None,
            ci_high: None,
            flags,
            note: row.footnote.clone(),
        }
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_rate_and_count() {
        let obs = normalize(Row {
            year_of_death: "2026".into(),
            month_of_death: "3".into(),
            maternal_deaths: Some("549".into()),
            maternal_mortality_rate: Some("15.3".into()),
            footnote: None,
        });
        assert_eq!(obs.len(), 2);
        assert_eq!(
            (obs[0].measure.as_str(), obs[0].value),
            ("maternal_mortality_rate", Some(15.3))
        );
        assert_eq!(
            (obs[1].measure.as_str(), obs[1].value),
            ("maternal_deaths", Some(549.0))
        );
        assert_eq!(obs[0].period.to_string(), "2026-03");
    }
}
