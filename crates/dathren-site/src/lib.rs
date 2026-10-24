mod chart;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use dathren_core::{Client, DatasetMeta, Observation, Query, export, registry};
use minijinja::{Environment, Value, context, path_loader};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Config {
    title: String,
    tagline: String,
    base_url: String,
    repo_url: Option<String>,
    giscus: Option<Giscus>,
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
    headline: Option<Headline>,
    chart: Option<chart::LineChart>,
}

#[derive(Serialize)]
struct Headline {
    label: &'static str,
    period: String,
    value: String,
    reported: Option<String>,
}

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

    let mut pages = Vec::new();
    for source in registry::all() {
        let meta = source.meta();
        let observations = source
            .fetch(
                client,
                &Query {
                    geo: Some("US".into()),
                    ..Query::default()
                },
            )
            .with_context(|| format!("fetching {}", meta.id))?;

        let data_dir = out.join("data").join(meta.id);
        fs::create_dir_all(&data_dir)?;
        fs::write(data_dir.join("us.json"), serde_json::to_vec(&observations)?)?;
        export::write_csv(fs::File::create(data_dir.join("us.csv"))?, &observations)?;

        let primary: Vec<&Observation> = match meta.measures.first() {
            Some(m) => observations.iter().filter(|o| o.measure == m.id).collect(),
            None => Vec::new(),
        };
        pages.push(DatasetPage {
            meta,
            headline: headline(meta, &primary),
            chart: chart::LineChart::new(&primary),
        });
    }

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
        ("discuss.html", "discuss"),
    ] {
        let dir = out.join(dir);
        fs::create_dir_all(&dir)?;
        let html = env
            .get_template(template)?
            .render(context! { site => &config, pages => &pages })?;
        fs::write(dir.join("index.html"), html)?;
    }
    Ok(())
}

fn headline(meta: &DatasetMeta, observations: &[&Observation]) -> Option<Headline> {
    let latest = observations.iter().rev().find(|o| o.best().is_some())?;
    Some(Headline {
        label: meta.measures.first()?.label,
        period: latest.period.to_string(),
        value: chart::thousands(latest.best()?),
        reported: latest
            .adjusted_value
            .and(latest.value)
            .map(chart::thousands),
    })
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
    fn inline_code_escapes_then_wraps() {
        assert_eq!(
            inline_code("use `value` <not> this").to_string(),
            "use <code>value</code> &lt;not&gt; this"
        );
    }
}
