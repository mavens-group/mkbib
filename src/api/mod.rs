// src/api/mod.rs
use anyhow::{anyhow, Result};
use biblatex::Bibliography;
use reqwest::header::ACCEPT;

pub async fn fetch_doi(doi: &str) -> Result<Bibliography> {
  let client = reqwest::Client::new();
  let url = format!("https://doi.org/{}", doi);

  let resp = client
    .get(&url)
    .header(ACCEPT, "application/x-bibtex")
    .send()
    .await?
    .text()
    .await?;

  // Parse the raw text into our Bibliography struct
  Bibliography::parse(&resp).map_err(|e| anyhow!("Parse Error: {}", e))
}

pub async fn search_crossref(query: &str) -> Result<Bibliography> {
  let client = reqwest::Client::new();
  let search_url = "https://api.crossref.org/works";

  // Simple query parameters for Crossref API
  let params = [("query", query), ("rows", "1")];

  let resp = client
    .get(search_url)
    .query(&params)
    .send()
    .await?
    .json::<serde_json::Value>()
    .await?;

  // Extract the DOI from the first result
  let doi = resp["message"]["items"][0]["DOI"]
    .as_str()
    .ok_or_else(|| anyhow!("No results found"))?;

  // Reuse our fetch_doi function
  fetch_doi(doi).await
}
