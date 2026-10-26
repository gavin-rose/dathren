mod changes;
mod chart;
mod discussions;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use changes::Change;
use discussions::Discussion;

use anyhow::{Context, Result};
use dathren::geo::{self, State};
use dathren::{Client, DatasetMeta, Geography, Observation, Query, Source, export, registry};
use minijinja::{Environment, Value, context, path_loader};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Config {
    title: String,
    tagline: String,
    base_url: String,
    site_url: Option<String>,
    repo_url: Option<String>,
    giscus: Option<Giscus>,
    #[serde(default)]
    discuss: Vec<DiscussCategory>,
}

#[derive(Deserialize, Serialize)]
struct DiscussCategory {
    slug: String,
    name: String,
    icon: String,
    description: String,
    github_slug: String,
    category_id: String,
}

#[derive(Serialize)]
struct CategoryPage<'a> {
    category: &'a DiscussCategory,
    discussions: Vec<Discussion>,
}

/// Discussion threads backed by GitHub Discussions. See <https://giscus.app>.
#[derive(Deserialize, Serialize)]
struct Giscus {
    repo: String,
    repo_id: String,
    category: String,
    category_id: String,
}

#[derive(Serialize)]
struct DatasetPage<'a> {
    meta: &'a DatasetMeta,
    unit: &'static str,
    /// `us` for datasets with a national series, `latest` for the newest period everywhere.
    download: &'static str,
    /// States with their own data file, or with counties in `latest`.
    states: Vec<&'static State>,
    headline: Option<Headline>,
    chart: Option<chart::LineChart>,
    ranking: Option<Ranking>,
    recent_changes: Vec<Change>,
}

#[derive(Serialize)]
struct Headline {
    label: &'static str,
    period: String,
    value: String,
    reported: Option<String>,
}

/// The newest period of the primary measure, for datasets without a national series.
#[derive(Serialize)]
struct Ranking {
    label: &'static str,
    level: &'static str,
    period: String,
    reported: usize,
    reported_label: String,
    rows: Vec<RankRow>,
}

#[derive(Serialize)]
struct RankRow {
    name: String,
    value: String,
    ci: Option<String>,
}

const COUNTY_RANKING_ROWS: usize = 15;

pub fn build(src: &Path, out: &Path, client: &Client) -> Result<()> {
    let config_path = src.join("config.toml");
    let config: Config = toml::from_str(
        &fs::read_to_string(&config_path)
            .with_context(|| format!("reading {}", config_path.display()))?,
    )?;

    let mut env = Environment::new();
    env.set_loader(path_loader(src.join("templates")));
    env.add_filter("inline_code", inline_code);

    if out.exists() {
        fs::remove_dir_all(out)?;
    }
    fs::create_dir_all(out)?;
    copy_dir(&src.join("static"), out)?;

    let previous = Previous::new(config.site_url.as_deref())?;
    let mut pages = Vec::new();
    let mut new_changes = Vec::new();
    for source in registry::all() {
        let (page, found) = dataset_page(
            *source,
            client,
            &out.join("data").join(source.meta().id),
            &previous,
        )
        .with_context(|| format!("building {}", source.meta().id))?;
        pages.push(page);
        new_changes.extend(found);
    }

    let mut history: Vec<Change> = previous.json("changes.json")?.unwrap_or_default();
    history.splice(0..0, new_changes);
    history.truncate(changes::MAX_CHANGES);
    fs::write(
        out.join("changes.json"),
        serde_json::to_vec_pretty(&history)?,
    )?;
    for page in &mut pages {
        page.recent_changes = history
            .iter()
            .filter(|c| c.dataset == page.meta.id)
            .take(3)
            .cloned()
            .collect();
    }
    let dir = out.join("changes");
    fs::create_dir_all(&dir)?;
    let html = env
        .get_template("changes.html")?
        .render(context! { site => &config, changes => &history })?;
    fs::write(dir.join("index.html"), html)?;
    let feed = env
        .get_template("changes.xml")?
        .render(context! { site => &config, changes => &history[..history.len().min(50)], updated => previous.date })?;
    fs::write(out.join("changes.xml"), feed)?;

    for page in &pages {
        let dir = out.join("datasets").join(page.meta.id);
        fs::create_dir_all(&dir)?;
        let html = env
            .get_template("dataset.html")?
            .render(context! { site => &config, page => page })?;
        fs::write(dir.join("index.html"), html)?;
    }
    for (template, dir) in [
        ("index.html", ""),
        ("datasets.html", "datasets"),
        ("codes.html", "codes"),
        ("explore.html", "explore"),
    ] {
        let dir = out.join(dir);
        fs::create_dir_all(&dir)?;
        let html = env
            .get_template(template)?
            .render(context! { site => &config, pages => &pages, states => geo::STATES })?;
        fs::write(dir.join("index.html"), html)?;
    }

    let token = std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty());
    let mut categories = Vec::new();
    for category in &config.discuss {
        let discussions = match (&token, &config.giscus) {
            (Some(token), Some(giscus)) => {
                discussions::fetch(&giscus.repo, &category.category_id, token)
                    .with_context(|| format!("fetching {} discussions", category.name))?
            }
            _ => Vec::new(),
        };
        categories.push(CategoryPage {
            category,
            discussions,
        });
    }
    let live = token.is_some();
    let mut recent: Vec<(&DiscussCategory, &Discussion)> = categories
        .iter()
        .flat_map(|c| c.discussions.iter().map(move |d| (c.category, d)))
        .collect();
    recent.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
    recent.truncate(5);
    let recent: Vec<_> = recent
        .into_iter()
        .map(|(category, discussion)| context! { category, discussion })
        .collect();

    let dir = out.join("discuss");
    fs::create_dir_all(&dir)?;
    let html = env
        .get_template("discuss.html")?
        .render(context! { site => &config, categories => &categories, recent, live })?;
    fs::write(dir.join("index.html"), html)?;
    for page in &categories {
        let dir = out.join("discuss").join(&page.category.slug);
        fs::create_dir_all(&dir)?;
        let html = env
            .get_template("discuss_category.html")?
            .render(context! { site => &config, page => page, live })?;
        fs::write(dir.join("index.html"), html)?;
        for discussion in &page.discussions {
            let dir = out.join("discuss/t").join(discussion.number.to_string());
            fs::create_dir_all(&dir)?;
            let html = env
                .get_template("discussion.html")?
                .render(context! { site => &config, category => page.category, discussion })?;
            fs::write(dir.join("index.html"), html)?;
        }
    }
    Ok(())
}

/// The currently deployed site, used to find what changed since the last build.
struct Previous {
    http: reqwest::blocking::Client,
    base: Option<String>,
    date: String,
}

impl Previous {
    fn new(site_url: Option<&str>) -> Result<Self> {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        Ok(Self {
            http: reqwest::blocking::Client::builder()
                .user_agent("dathren-site")
                .timeout(std::time::Duration::from_secs(60))
                .build()?,
            base: site_url.map(|u| u.trim_end_matches('/').to_string()),
            date: changes::date_from_unix(secs),
        })
    }

    /// `None` when the file isn't deployed yet. Network errors fail the build so a
    /// hiccup can't erase the change history.
    fn json<T: DeserializeOwned>(&self, path: &str) -> Result<Option<T>> {
        let Some(base) = &self.base else {
            return Ok(None);
        };
        let url = format!("{base}/{path}");
        let response = self
            .http
            .get(&url)
            .send()
            .with_context(|| format!("fetching {url}"))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let body = response.error_for_status()?.text()?;
        match serde_json::from_str(&body) {
            Ok(value) => Ok(Some(value)),
            Err(e) => {
                eprintln!("warning: ignoring unreadable {url}: {e}");
                Ok(None)
            }
        }
    }

    fn data(&self, dataset: &str, file: &str) -> Result<Vec<Observation>> {
        Ok(self
            .json(&format!("data/{dataset}/{file}.json"))?
            .unwrap_or_default())
    }
}

fn dataset_page<'a>(
    source: &'a dyn Source,
    client: &Client,
    data_dir: &Path,
    previous: &Previous,
) -> Result<(DatasetPage<'a>, Vec<Change>)> {
    let meta = source.meta();
    let primary = meta.measures.first().context("dataset has no measures")?;
    fs::create_dir_all(data_dir)?;
    let write = |name: &str, observations: &[Observation]| -> Result<()> {
        fs::write(
            data_dir.join(format!("{name}.json")),
            serde_json::to_vec(observations)?,
        )?;
        export::write_csv(
            fs::File::create(data_dir.join(format!("{name}.csv")))?,
            observations,
        )?;
        Ok(())
    };

    if meta.has(Geography::Nation) {
        let mut by_geo: BTreeMap<String, Vec<Observation>> = BTreeMap::new();
        for o in source.fetch(client, &Query::default())? {
            by_geo.entry(o.geo.clone()).or_default().push(o);
        }
        let observations = by_geo.remove("US").unwrap_or_default();
        write("us", &observations)?;
        let state_dir = data_dir.join("state");
        fs::create_dir_all(&state_dir)?;
        let mut states = Vec::new();
        for (code, rows) in &by_geo {
            if let Some(state) = geo::state(code) {
                fs::write(
                    state_dir.join(format!("{code}.json")),
                    serde_json::to_vec(rows)?,
                )?;
                states.push(state);
            }
        }
        states.sort_by_key(|s| s.name);

        let mut old = previous.data(meta.id, "us")?;
        for state in &states {
            old.extend(previous.data(meta.id, &format!("state/{}", state.abbr))?);
        }
        let current: Vec<Observation> = observations
            .iter()
            .chain(by_geo.values().flatten())
            .cloned()
            .collect();
        let found = changes::diff(meta, &previous.date, &old, &current);

        let series: Vec<&Observation> = observations
            .iter()
            .filter(|o| o.measure == primary.id)
            .collect();
        let page = DatasetPage {
            meta,
            unit: primary.unit,
            download: "us",
            states,
            headline: headline(primary.label, primary.unit, &series),
            chart: chart::LineChart::new(&series, primary.unit),
            ranking: None,
            recent_changes: Vec::new(),
        };
        return Ok((page, found));
    }

    let recent = Query {
        measures: vec![primary.id.into()],
        since_year: Some(current_year() - 2),
        ..Query::default()
    };
    let mut observations = source.fetch(client, &recent)?;
    if observations.is_empty() {
        let all = Query {
            measures: vec![primary.id.into()],
            ..Query::default()
        };
        observations = source.fetch(client, &all)?;
    }
    let newest = observations.iter().map(|o| o.period).max();
    observations.retain(|o| Some(o.period) == newest);
    write("latest", &observations)?;
    let found = changes::diff(
        meta,
        &previous.date,
        &previous.data(meta.id, "latest")?,
        &observations,
    );

    let county = meta.has(Geography::County);
    let mut states: Vec<&'static State> = geo::STATES
        .iter()
        .filter(|s| {
            observations.iter().any(|o| {
                if county {
                    o.geo.len() == 5 && o.geo.starts_with(s.fips)
                } else {
                    o.geo == s.abbr
                }
            })
        })
        .collect();
    states.sort_by_key(|s| s.name);
    let mut reported: Vec<&Observation> = observations
        .iter()
        .filter(|o| o.value.is_some() && o.geo != "US")
        .collect();
    reported.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
    let shown = if county {
        COUNTY_RANKING_ROWS
    } else {
        reported.len()
    };
    let rows = reported
        .iter()
        .take(shown)
        .map(|o| RankRow {
            name: o.geo_name.clone().unwrap_or_else(|| o.geo.clone()),
            value: chart::format_value(o.value.unwrap(), primary.unit),
            ci: o.ci_low.zip(o.ci_high).map(|(lo, hi)| {
                format!(
                    "{} to {}",
                    chart::format_value(lo, primary.unit),
                    chart::format_value(hi, primary.unit)
                )
            }),
        })
        .collect();
    let page = DatasetPage {
        meta,
        unit: primary.unit,
        download: "latest",
        states,
        headline: None,
        chart: None,
        ranking: newest.map(|period| Ranking {
            label: primary.label,
            level: if county { "county" } else { "state" },
            period: period.to_string(),
            reported: reported.len(),
            reported_label: chart::thousands(reported.len() as f64),
            rows,
        }),
        recent_changes: Vec::new(),
    };
    Ok((page, found))
}

fn headline(label: &'static str, unit: &str, series: &[&Observation]) -> Option<Headline> {
    let latest = series.iter().rev().find(|o| o.best().is_some())?;
    Some(Headline {
        label,
        period: latest.period.to_string(),
        value: chart::format_value(latest.best()?, unit),
        reported: latest
            .adjusted_value
            .and(latest.value)
            .map(|v| chart::format_value(v, unit)),
    })
}

fn current_year() -> i32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    1970 + (secs / 31_556_952) as i32
}

/// Escapes text and turns `backticked` spans into `<code>` elements.
fn inline_code(text: &str) -> Value {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    let html = escaped
        .split('`')
        .enumerate()
        .map(|(i, part)| {
            if i % 2 == 1 {
                format!("<code>{part}</code>")
            } else {
                part.to_owned()
            }
        })
        .collect::<String>();
    Value::from_safe_string(html)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target)?;
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discussion_pages_render() {
        let config: Config =
            toml::from_str(&fs::read_to_string("../../site/config.toml").unwrap()).unwrap();
        let mut env = Environment::new();
        env.set_loader(path_loader("../../site/templates"));
        let category = &config.discuss[0];
        let discussion = discussions::tests::sample();

        let html = env
            .get_template("discussion.html")
            .unwrap()
            .render(context! { site => &config, category, discussion => &discussion })
            .unwrap();
        assert!(html.contains("Why do 2026 counts look low?"));
        assert!(html.contains("<strong>low</strong>"));
        assert!(html.contains(r#"data-mapping="number" data-term="7""#));

        let page = CategoryPage {
            category,
            discussions: vec![discussion],
        };
        let html = env
            .get_template("discuss_category.html")
            .unwrap()
            .render(context! { site => &config, page => &page, live => true })
            .unwrap();
        assert!(html.contains("discuss/t/7/"));
        assert!(html.contains("3 replies"));
    }

    #[test]
    fn inline_code_escapes_then_wraps() {
        assert_eq!(
            inline_code("use `value` <not> this").to_string(),
            "use <code>value</code> &lt;not&gt; this"
        );
    }
}
