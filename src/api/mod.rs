// src/api/mod.rs
use anyhow::{anyhow, Result};
use biblatex::Bibliography;
use reqwest::header::ACCEPT;
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;

fn http_client() -> Result<&'static reqwest::Client> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("mkbib/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let _ = CLIENT.set(client);
    CLIENT
        .get()
        .ok_or_else(|| anyhow!("failed to initialize HTTP client"))
}

// Data structure for a single search result
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResultItem {
    pub title: String,
    pub author: String,
    pub year: String,
    pub doi: String,
}

#[derive(Deserialize)]
struct CrossrefResponse {
    message: CrossrefMessage,
}

#[derive(Deserialize)]
struct CrossrefMessage {
    #[serde(default)]
    items: Vec<CrossrefWork>,
}

#[derive(Deserialize)]
struct CrossrefWork {
    #[serde(default)]
    title: Vec<String>,
    #[serde(rename = "DOI", default)]
    doi: String,
    published: Option<CrossrefDate>,
    #[serde(default)]
    author: Vec<CrossrefAuthor>,
}

#[derive(Deserialize)]
struct CrossrefDate {
    #[serde(rename = "date-parts", default)]
    date_parts: Vec<Vec<i32>>,
}

#[derive(Deserialize)]
struct CrossrefAuthor {
    family: Option<String>,
    given: Option<String>,
}

// Fetch a single BibTeX entry by DOI
pub async fn fetch_doi(doi: &str) -> Result<Bibliography> {
    let client = http_client()?;
    let doi = doi
        .trim()
        .strip_prefix("https://doi.org/")
        .unwrap_or(doi.trim());
    let url = reqwest::Url::parse(&format!("https://doi.org/{doi}"))?;

    let resp = client
        .get(url)
        .header(ACCEPT, "application/x-bibtex")
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let resp = crate::core::normalize_month_macros(&resp);
    Bibliography::parse(&resp).map_err(|e| anyhow!("Parse Error: {}", e))
}

// Fetch a list of suggestions (Title, Author, Year)
pub async fn search_crossref_suggestions(query: &str) -> Result<Vec<SearchResultItem>> {
    let client = http_client()?;
    let search_url = "https://api.crossref.org/works";

    let params = [("query", query), ("rows", "10")];

    let resp = client
        .get(search_url)
        .query(&params)
        .send()
        .await?
        .error_for_status()?
        .json::<CrossrefResponse>()
        .await?;

    let mut results = Vec::new();

    for item in resp.message.items {
        let title = item
            .title
            .into_iter()
            .next()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "No Title".to_string());
        let year = item
            .published
            .and_then(|date| date.date_parts.into_iter().next())
            .and_then(|parts| parts.into_iter().next())
            .map(|year| year.to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let author = item
            .author
            .into_iter()
            .take(3)
            .filter_map(|author| {
                let family = author.family?;
                Some(
                    format!("{} {}", author.given.unwrap_or_default(), family)
                        .trim()
                        .to_string(),
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let author = if author.is_empty() {
            "Unknown Author".to_string()
        } else {
            author
        };

        if !item.doi.is_empty() {
            results.push(SearchResultItem {
                title,
                author,
                year,
                doi: item.doi,
            });
        }
    }

    Ok(results)
}
