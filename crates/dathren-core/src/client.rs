use std::time::Duration;

use crate::Result;

pub struct Client {
    pub(crate) http: reqwest::blocking::Client,
}

impl Client {
    pub fn new() -> Result<Self> {
        let http = reqwest::blocking::Client::builder()
            .user_agent(concat!("dathren/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(Self { http })
    }
}
