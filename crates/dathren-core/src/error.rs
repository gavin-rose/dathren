#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unknown dataset `{0}`")]
    UnknownDataset(String),
    #[error("unknown measure `{measure}` for dataset `{dataset}`")]
    UnknownMeasure { dataset: String, measure: String },
}

pub type Result<T> = std::result::Result<T, Error>;
