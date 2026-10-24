use crate::{Client, DatasetMeta, Observation, Result};

/// Filters applied when fetching. Empty fields mean "everything".
#[derive(Debug, Default, Clone)]
pub struct Query {
    /// Geography code, e.g. `US` or a state abbreviation.
    pub geo: Option<String>,
    pub measures: Vec<String>,
    pub since_year: Option<i32>,
}

pub trait Source: Sync {
    fn meta(&self) -> &'static DatasetMeta;
    fn fetch(&self, client: &Client, query: &Query) -> Result<Vec<Observation>>;
}
