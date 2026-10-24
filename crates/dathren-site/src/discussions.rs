use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;

const QUERY: &str = r#"
query($owner: String!, $name: String!, $category: ID!) {
  repository(owner: $owner, name: $name) {
    discussions(first: 50, categoryId: $category, orderBy: {field: UPDATED_AT, direction: DESC}) {
      nodes {
        number title url createdAt updatedAt bodyHTML bodyText upvoteCount
        author { login avatarUrl }
        comments { totalCount }
      }
    }
  }
}"#;

#[derive(Deserialize, Serialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Discussion {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(rename(deserialize = "bodyHTML"))]
    pub body_html: String,
    pub body_text: String,
    #[serde(skip_deserializing)]
    pub excerpt: String,
    pub upvote_count: u64,
    pub author: Option<Author>,
    pub comments: Count,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Author {
    pub login: String,
    pub avatar_url: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Count {
    pub total_count: u64,
}

pub fn fetch(repo: &str, category_id: &str, token: &str) -> Result<Vec<Discussion>> {
    let (owner, name) = repo
        .split_once('/')
        .context("giscus.repo must be owner/name")?;
    let response: serde_json::Value = reqwest::blocking::Client::new()
        .post("https://api.github.com/graphql")
        .bearer_auth(token)
        .header("User-Agent", "dathren")
        .json(&json!({
            "query": QUERY,
            "variables": { "owner": owner, "name": name, "category": category_id },
        }))
        .send()?
        .error_for_status()?
        .json()?;
    if let Some(errors) = response.get("errors") {
        bail!("GitHub API error: {errors}");
    }
    let nodes = response["data"]["repository"]["discussions"]["nodes"].clone();
    let mut discussions: Vec<Discussion> = serde_json::from_value(nodes)?;
    for d in &mut discussions {
        d.excerpt = excerpt(&d.body_text, 180);
    }
    Ok(discussions)
}

fn excerpt(text: &str, max_chars: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max_chars {
        return text;
    }
    let cut: String = text.chars().take(max_chars).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!(
        "{}…",
        cut.trim_end_matches(|c: char| c.is_ascii_punctuation())
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn sample() -> Discussion {
        serde_json::from_value(json!({
            "number": 7,
            "title": "Why do 2026 counts look low?",
            "url": "https://github.com/gavin-rose/dathren/discussions/7",
            "createdAt": "2026-10-20T14:03:00Z",
            "updatedAt": "2026-10-22T09:15:00Z",
            "bodyHTML": "<p>Recent months seem <strong>low</strong>.</p>",
            "bodyText": "Recent months seem low.",
            "upvoteCount": 2,
            "author": { "login": "octocat", "avatarUrl": "https://avatars.githubusercontent.com/u/583231" },
            "comments": { "totalCount": 3 }
        }))
        .unwrap()
    }

    #[test]
    fn excerpts_cut_on_word_boundaries() {
        assert_eq!(excerpt("short   text\nhere", 50), "short text here");
        assert_eq!(excerpt("one two three four", 10), "one two…");
    }

    #[test]
    fn parses_graphql_nodes() {
        let d = sample();
        assert_eq!(d.number, 7);
        assert_eq!(
            d.body_html,
            "<p>Recent months seem <strong>low</strong>.</p>"
        );
        assert_eq!(d.author.unwrap().login, "octocat");
        assert_eq!(d.comments.total_count, 3);
    }
}
