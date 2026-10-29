//! Health indicators by country from the World Bank's World Development Indicators,
//! many of them compiled from WHO and the UN.
//! <https://data.worldbank.org/topic/health>

use std::collections::HashMap;

use serde::Deserialize;

use crate::{
    Caveat, Client, DatasetMeta, Geography, Measure, Observation, Period, Query, Result, Source,
};

pub struct WorldBankHealth;

const ID: &str = "world-bank-health";
const API: &str = "https://api.worldbank.org/v2";
const FIRST_YEAR: i32 = 2000;

/// Measure ids paired with World Bank indicator codes.
const INDICATORS: &[(&str, &str)] = &[
    ("life_expectancy", "SP.DYN.LE00.IN"),
    ("suicide_rate", "SH.STA.SUIC.P5"),
    ("alcohol_per_capita", "SH.ALC.PCAP.LI"),
    ("tobacco_use", "SH.PRV.SMOK"),
    ("overweight", "SH.STA.OWAD.ZS"),
    ("maternal_mortality", "SH.STA.MMRT"),
    ("under5_mortality", "SH.DYN.MORT"),
    ("road_traffic_deaths", "SH.STA.TRAF.P5"),
    ("health_spending", "SH.XPD.CHEX.GD.ZS"),
    ("physicians", "SH.MED.PHYS.ZS"),
];

#[rustfmt::skip]
static META: DatasetMeta = DatasetMeta {
    id: ID,
    title: "Global Health Indicators by Country",
    publisher: "World Bank, compiled from WHO, the UN, and other agencies",
    summary: "Life expectancy, suicide, alcohol and tobacco use, maternal and child mortality, \
              and health spending for about 200 countries, yearly since 2000.",
    source_url: "https://data.worldbank.org/topic/health",
    update_cadence: "Several times a year",
    topics: &["global", "mortality", "mental health", "substance use"],
    geographies: &[Geography::Country],
    period_label: "Year",
    series_note: "Latest available year for each country.",
    measures: &[
        Measure { id: "life_expectancy", label: "Life expectancy at birth", code: None, unit: "years" },
        Measure { id: "suicide_rate", label: "Suicide mortality rate", code: None, unit: "deaths per 100,000" },
        Measure { id: "alcohol_per_capita", label: "Alcohol consumption, adults 15+", code: None, unit: "liters of pure alcohol per person" },
        Measure { id: "tobacco_use", label: "Current tobacco use, adults", code: None, unit: "percent" },
        Measure { id: "overweight", label: "Overweight, adults", code: None, unit: "percent" },
        Measure { id: "maternal_mortality", label: "Maternal mortality ratio (modeled)", code: None, unit: "deaths per 100,000 live births" },
        Measure { id: "under5_mortality", label: "Under-5 mortality", code: None, unit: "deaths per 1,000 live births" },
        Measure { id: "road_traffic_deaths", label: "Road traffic deaths", code: None, unit: "deaths per 100,000" },
        Measure { id: "health_spending", label: "Current health spending", code: None, unit: "percent of GDP" },
        Measure { id: "physicians", label: "Physicians", code: None, unit: "per 1,000 people" },
    ],
    caveats: &[
        Caveat {
            title: "Each indicator has its own source",
            detail: "The World Bank republishes figures from WHO, the UN, and others, and each \
                     source has its own methods and update schedule. The source is named on each \
                     indicator's World Bank page.",
        },
        Caveat {
            title: "Many values are model estimates",
            detail: "Maternal and child mortality, suicide, and alcohol figures are often \
                     estimated, especially where death registration is incomplete. Treat small \
                     differences between countries with care.",
        },
        Caveat {
            title: "The latest year differs by country",
            detail: "Some countries report years later than others, so a map of the latest values \
                     mixes years. Check `period` before comparing countries.",
        },
        Caveat {
            title: "Values get revised",
            detail: "New releases can change earlier years, not just add new ones.",
        },
        Caveat {
            title: "Reuse terms",
            detail: "World Bank data is licensed CC BY 4.0, with credit to the World Bank. Some \
                     indicators come from WHO and may also carry WHO's terms of use.",
        },
    ],
};

#[derive(Deserialize)]
struct Country {
    id: String,
    name: String,
    region: Named,
}

#[derive(Deserialize)]
struct Named {
    value: String,
}

#[derive(Deserialize)]
struct Row {
    countryiso3code: String,
    date: String,
    value: Option<f64>,
}

#[derive(Deserialize)]
struct Page {
    pages: u32,
}

impl Source for WorldBankHealth {
    fn meta(&self) -> &'static DatasetMeta {
        &META
    }

    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>> {
        META.check_measures(&query.measures)?;
        let countries: HashMap<String, String> =
            get_pages::<Country>(client, &format!("{API}/country"), &[])?
                .into_iter()
                .filter(|c| c.region.value != "Aggregates")
                .map(|c| (c.id, c.name))
                .collect();
        let place = query
            .geo
            .as_deref()
            .map_or("all".to_string(), str::to_uppercase);
        let years = format!("{}:2100", query.since_year.unwrap_or(FIRST_YEAR));

        let mut observations = Vec::new();
        for (measure, code) in INDICATORS {
            if !query.measures.is_empty() && !query.measures.iter().any(|m| m == measure) {
                continue;
            }
            let url = format!("{API}/country/{place}/indicator/{code}");
            for row in get_pages::<Row>(client, &url, &[("date", years.as_str())])? {
                if let Some(o) = normalize(measure, row, &countries) {
                    observations.push(o);
                }
            }
        }
        observations
            .sort_by(|a, b| (&a.measure, &a.geo, a.period).cmp(&(&b.measure, &b.geo, b.period)));
        Ok(observations)
    }
}

/// The World Bank API returns `[paging, rows]`, or `[message]` for an unknown country.
fn get_pages<T: serde::de::DeserializeOwned>(
    client: &Client,
    url: &str,
    params: &[(&str, &str)],
) -> Result<Vec<T>> {
    let mut rows = Vec::new();
    let mut page = 1;
    loop {
        let page_param = page.to_string();
        let mut query = vec![
            ("format", "json"),
            ("per_page", "20000"),
            ("page", page_param.as_str()),
        ];
        query.extend_from_slice(params);
        let body: Vec<serde_json::Value> = client
            .http
            .get(url)
            .query(&query)
            .send()?
            .error_for_status()?
            .json()?;
        let (Some(paging), Some(data)) = (body.first(), body.get(1)) else {
            return Ok(rows);
        };
        let pages = serde_json::from_value::<Page>(paging.clone()).map_or(1, |p| p.pages);
        if let Ok(batch) = serde_json::from_value::<Vec<T>>(data.clone()) {
            rows.extend(batch);
        }
        if page >= pages {
            return Ok(rows);
        }
        page += 1;
    }
}

fn normalize(measure: &str, row: Row, countries: &HashMap<String, String>) -> Option<Observation> {
    let name = countries.get(&row.countryiso3code)?;
    let value = row.value?;
    Some(Observation {
        dataset: ID.into(),
        measure: measure.into(),
        geo: row.countryiso3code,
        geo_name: Some(name.clone()),
        period: Period::year(row.date.parse().ok()?),
        value: Some(value),
        adjusted_value: None,
        ci_low: None,
        ci_high: None,
        flags: Vec::new(),
        note: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_countries_and_skips_aggregates_and_gaps() {
        let countries = HashMap::from([("JPN".to_string(), "Japan".to_string())]);
        let row = |iso: &str, value: Option<f64>| Row {
            countryiso3code: iso.into(),
            date: "2024".into(),
            value,
        };
        let obs = normalize("life_expectancy", row("JPN", Some(84.7)), &countries).unwrap();
        assert_eq!(
            (obs.geo.as_str(), obs.geo_name.as_deref(), obs.value),
            ("JPN", Some("Japan"), Some(84.7))
        );
        assert!(normalize("life_expectancy", row("AFE", Some(64.5)), &countries).is_none());
        assert!(normalize("life_expectancy", row("JPN", None), &countries).is_none());
    }
}
