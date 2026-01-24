// src/api/mod.rs
pub mod doi;
use anyhow::{anyhow, Result};
use biblatex::Bibliography;
use reqwest::header::ACCEPT;

pub async fn fetch_doi(doi: &str) -> Result<Bibliography> {
  // ... (Keep existing code)
  let client = reqwest::Client::new();
  let url = format!("https://doi.org/{}", doi);

  let resp = client
    .get(&url)
    .header(ACCEPT, "application/x-bibtex")
    .send()
    .await?
    .text()
    .await?;

  Bibliography::parse(&resp).map_err(|e| anyhow!("Parse Error: {}", e))
}

pub async fn search_crossref(query: &str) -> Result<Bibliography> {
  // ... (Keep existing code)
  let client = reqwest::Client::new();
  let search_url = "https://api.crossref.org/works";
  let params = [("query", query), ("rows", "1")];

  let resp = client
    .get(search_url)
    .query(&params)
    .send()
    .await?
    .json::<serde_json::Value>()
    .await?;

  let doi = resp["message"]["items"][0]["DOI"]
    .as_str()
    .ok_or_else(|| anyhow!("No results found"))?;

  fetch_doi(doi).await
}
