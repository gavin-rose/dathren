use serde::de::DeserializeOwned;

use crate::{Client, Result};

const PAGE_SIZE: usize = 50_000;

pub fn fetch_all<T: DeserializeOwned>(
    client: &Client,
    domain: &str,
    resource: &str,
    where_clause: &str,
) -> Result<Vec<T>> {
    let url = format!("https://{domain}/resource/{resource}.json");
    let mut rows = Vec::new();
    loop {
        let offset = rows.len().to_string();
        let limit = PAGE_SIZE.to_string();
        let page: Vec<T> = client
            .http
            .get(&url)
            .query(&[
                ("$where", where_clause),
                ("$order", ":id"),
                ("$limit", &limit),
                ("$offset", &offset),
            ])
            .send()?
            .error_for_status()?
            .json()?;
        let done = page.len() < PAGE_SIZE;
        rows.extend(page);
        if done {
            return Ok(rows);
        }
    }
}

pub fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
