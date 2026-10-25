use dathren::{Client, Query, export, registry};
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
