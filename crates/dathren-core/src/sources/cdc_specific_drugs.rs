//! CDC provisional drug overdose death counts for specific drugs.
//! <https://data.cdc.gov/d/8hzs-zshh>

use serde::Deserialize;

use super::number;
use super::socrata::{self, quote};
use crate::{
    Caveat, Client, DatasetMeta, Flag, Geography, Measure, Observation, Period, Query, Result,
    Source,
};

pub struct CdcSpecificDrugs;

const ID: &str = "cdc-overdose-specific-drugs";

/// Measure ids paired with the exact upstream `drug_involved` strings.
const DRUGS: &[(&str, &str)] = &[
    ("fentanyl", "Fentanyl"),
    ("methamphetamine", "Methamphetamine"),
    ("cocaine", "Cocaine"),
    ("heroin", "Heroin"),
    ("oxycodone", "Oxycodone"),
    ("xylazine", "Xylazine"),
    ("amphetamine", "Amphetamine"),
    ("gabapentin", "Gabapentin"),
    ("bromazolam", "Bromazolam"),
    ("mitragynine", "Mitragynine"),
    ("ketamine", "Ketamine"),
    ("mdma", "MDMA"),
];

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Overdose Deaths by Specific Drug",
    publisher: "CDC National Center for Health Statistics (VSRR)",
    summary: "Rolling 12-month counts of overdose deaths involving 12 specific drugs, including \
              fentanyl, xylazine, bromazolam, and kratom, found in death certificate text.",
    source_url: "https://www.cdc.gov/nchs/nvss/vsrr/drug-overdose-data.htm",
    update_cadence: "Quarterly",
    topics: &["substance use", "mortality", "emerging drugs"],
    geographies: &[Geography::Nation],
    period_label: "12 months ending",
    series_note: "Overdose deaths involving the drug in the 12 months ending each month.",
    measures: &[
        Measure { id: "fentanyl", label: "Fentanyl", code: None, unit: "deaths" },
        Measure { id: "methamphetamine", label: "Methamphetamine", code: None, unit: "deaths" },
        Measure { id: "cocaine", label: "Cocaine", code: None, unit: "deaths" },
        Measure { id: "heroin", label: "Heroin", code: None, unit: "deaths" },
        Measure { id: "oxycodone", label: "Oxycodone", code: None, unit: "deaths" },
        Measure { id: "xylazine", label: "Xylazine", code: None, unit: "deaths" },
        Measure { id: "amphetamine", label: "Amphetamine", code: None, unit: "deaths" },
        Measure { id: "gabapentin", label: "Gabapentin", code: None, unit: "deaths" },
        Measure { id: "bromazolam", label: "Bromazolam (a designer benzodiazepine)", code: None, unit: "deaths" },
        Measure { id: "mitragynine", label: "Mitragynine (kratom)", code: None, unit: "deaths" },
        Measure { id: "ketamine", label: "Ketamine", code: None, unit: "deaths" },
        Measure { id: "mdma", label: "MDMA (ecstasy)", code: None, unit: "deaths" },
    ],
    caveats: &[
        Caveat {
            title: "Drugs overlap",
            detail: "Most overdose deaths involve more than one drug, and each drug named counts \
                     the death. Don't add drugs together.",
        },
        Caveat {
            title: "Drugs are found in death certificate text",
            detail: "CDC searches the written cause of death, not lab results. Drugs that aren't \
                     routinely tested for, like xylazine in some places, are likely undercounted.",
        },
        Caveat {
            title: "Values are rolling 12-month totals",
            detail: "Each period is the 12 months ending in that month. Don't add consecutive \
                     periods together.",
        },
        Caveat {
            title: "Counts are provisional",
            detail: "Recent months will likely rise as investigations finish. Regional counts \
                     for HHS regions exist upstream but aren't included yet.",
        },
    ],
};

#[derive(Deserialize)]
struct Row {
    death_year: String,
    death_month: String,
    jurisdiction_occurrence: String,
    drug_involved: String,
    drug_overdose_deaths: Option<String>,
    footnote: Option<String>,
}

impl Source for CdcSpecificDrugs {
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
        let mut clauses = vec![format!(
            "jurisdiction_occurrence = {}",
            quote("United States")
        )];
        if let Some(year) = query.since_year {
            clauses.push(format!("death_year >= {}", quote(&year.to_string())));
        }
        if !query.measures.is_empty() {
            let drugs: Vec<String> = DRUGS
                .iter()
                .filter(|(id, _)| query.measures.iter().any(|m| m == id))
                .map(|(_, drug)| quote(drug))
                .collect();
            clauses.push(format!("drug_involved in ({})", drugs.join(", ")));
        }

        let rows: Vec<Row> =
            socrata::fetch_all(client, "data.cdc.gov", "8hzs-zshh", &clauses.join(" AND "))?;
        let mut observations: Vec<Observation> = rows.into_iter().filter_map(normalize).collect();
        observations.sort_by(|a, b| (&a.measure, a.period).cmp(&(&b.measure, b.period)));
        Ok(observations)
    }
}

fn normalize(row: Row) -> Option<Observation> {
    if row.jurisdiction_occurrence != "United States" {
        return None;
    }
    let measure = DRUGS.iter().find(|(_, d)| *d == row.drug_involved)?.0;
    let value = number(row.drug_overdose_deaths.as_deref());
    let mut flags = vec![Flag::Provisional];
    if value.is_none() {
        flags.push(Flag::Suppressed);
    }
    Some(Observation {
        dataset: ID.into(),
        measure: measure.into(),
        geo: "US".into(),
        geo_name: Some("United States".into()),
        period: Period::month(
            row.death_year.parse().ok()?,
            super::month_number(&row.death_month)?,
        ),
        value,
        adjusted_value: None,
        ci_low: None,
        ci_high: None,
        flags,
        note: row.footnote,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_national_rows_only() {
        let row = |jurisdiction: &str| Row {
            death_year: "2025".into(),
            death_month: "December".into(),
            jurisdiction_occurrence: jurisdiction.into(),
            drug_involved: "Xylazine".into(),
            drug_overdose_deaths: Some("2483".into()),
            footnote: None,
        };
        let obs = normalize(row("United States")).unwrap();
        assert_eq!(
            (obs.measure.as_str(), obs.value),
            ("xylazine", Some(2483.0))
        );
        assert_eq!(obs.period.to_string(), "2025-12");
        assert!(normalize(row("Region 3")).is_none());
    }
}
