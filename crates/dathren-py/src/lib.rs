use dathren::{Client, Error, Query, export, registry};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

fn py_err(e: Error) -> PyErr {
    match e {
        Error::UnknownDataset(_) | Error::UnknownMeasure { .. } => {
            PyValueError::new_err(e.to_string())
        }
        Error::Http(_) => PyRuntimeError::new_err(e.to_string()),
    }
}

fn json<T: serde::Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
fn datasets_json() -> PyResult<String> {
    json(&registry::all().iter().map(|s| s.meta()).collect::<Vec<_>>())
}

#[pyfunction]
fn info_json(dataset: &str) -> PyResult<String> {
    json(registry::get(dataset).map_err(py_err)?.meta())
}

#[pyfunction]
#[pyo3(signature = (dataset, geo=None, measures=Vec::new(), since=None))]
fn fetch_json(
    py: Python<'_>,
    dataset: &str,
    geo: Option<String>,
    measures: Vec<String>,
    since: Option<i32>,
) -> PyResult<String> {
    let source = registry::get(dataset).map_err(py_err)?;
    let query = Query {
        geo,
        measures,
        since_year: since,
    };
    let observations = py
        .detach(|| source.fetch(&Client::new()?, &query))
        .map_err(py_err)?;
    json(&export::records(&observations))
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(datasets_json, m)?)?;
    m.add_function(wrap_pyfunction!(info_json, m)?)?;
    m.add_function(wrap_pyfunction!(fetch_json, m)?)?;
    Ok(())
}
