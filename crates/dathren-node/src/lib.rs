use dathren::{Client, Query, export, registry, tools};
use napi::bindgen_prelude::*;
use napi::{Env, Task};
use napi_derive::napi;

fn json<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|e| Error::from_reason(e.to_string()))
}

fn js_err(e: dathren::Error) -> Error {
    Error::from_reason(e.to_string())
}

#[napi]
pub fn datasets_json() -> Result<String> {
    json(&registry::all().iter().map(|s| s.meta()).collect::<Vec<_>>())
}

#[napi]
pub fn info_json(dataset: String) -> Result<String> {
    json(registry::get(&dataset).map_err(js_err)?.meta())
}

#[napi(object)]
pub struct FetchOptions {
    pub geo: Option<String>,
    pub measures: Option<Vec<String>>,
    pub since: Option<i32>,
}

pub struct Fetch {
    dataset: String,
    query: Query,
}

impl Task for Fetch {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<String> {
        let source = registry::get(&self.dataset).map_err(js_err)?;
        let observations = source
            .fetch(&Client::new().map_err(js_err)?, &self.query)
            .map_err(js_err)?;
        json(&export::records(&observations))
    }

    fn resolve(&mut self, _env: Env, output: String) -> Result<String> {
        Ok(output)
    }
}

/// Runs on the libuv thread pool so the event loop stays free during the request.
#[napi]
pub fn fetch_json(dataset: String, options: Option<FetchOptions>) -> AsyncTask<Fetch> {
    let options = options.unwrap_or(FetchOptions {
        geo: None,
        measures: None,
        since: None,
    });
    AsyncTask::new(Fetch {
        dataset,
        query: Query {
            geo: options.geo,
            measures: options.measures.unwrap_or_default(),
            since_year: options.since,
        },
    })
}

#[napi]
pub fn cite_json(dataset: String, accessed: Option<String>) -> Result<String> {
    let meta = registry::get(&dataset).map_err(js_err)?.meta();
    json(&tools::cite(meta, &accessed.unwrap_or_else(tools::today)))
}

#[napi]
pub fn latest_json(rows: String) -> Result<String> {
    let observations = export::from_records_json(&rows).map_err(js_err)?;
    json(&export::records(&tools::latest(observations)))
}

#[napi]
pub fn total_json(rows: String, low: f64, high: f64) -> Result<String> {
    let observations = export::from_records_json(&rows).map_err(js_err)?;
    json(&tools::total(&observations, (low, high)).map_err(js_err)?)
}

pub struct PerCapita {
    rows: String,
    per: f64,
}

impl Task for PerCapita {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<String> {
        let observations = export::from_records_json(&self.rows).map_err(js_err)?;
        let client = Client::new().map_err(js_err)?;
        let rates = tools::per_capita_fetch(&client, &observations, self.per).map_err(js_err)?;
        json(&export::records(&rates))
    }

    fn resolve(&mut self, _env: Env, output: String) -> Result<String> {
        Ok(output)
    }
}

#[napi]
pub fn per_capita_json(rows: String, per: f64) -> AsyncTask<PerCapita> {
    AsyncTask::new(PerCapita { rows, per })
}
