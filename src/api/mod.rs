// src/api/mod.rs
use anyhow::{anyhow, Result};
use biblatex::Bibliography;
use reqwest::header::ACCEPT;
use serde::Deserialize;

// Data structure for a single search result
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResultItem {
  pub title: String,
  pub author: String,
  pub year: String,
  pub doi: String,
}

// Fetch a single BibTeX entry by DOI
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

  Bibliography::parse(&resp).map_err(|e| anyhow!("Parse Error: {}", e))
}

// Fetch a list of suggestions (Title, Author, Year)
pub async fn search_crossref_suggestions(query: &str) -> Result<Vec<SearchResultItem>> {
  let client = reqwest::Client::new();
  let search_url = "https://api.crossref.org/works";

  let params = [("query", query), ("rows", "10")];

  let resp = client
    .get(search_url)
    .query(&params)
    .send()
    .await?
    .json::<serde_json::Value>()
    .await?;

  let items = resp["message"]["items"]
    .as_array()
    .ok_or_else(|| anyhow!("No results found"))?;

  let mut results = Vec::new();

  for item in items {
    let title = item["title"]
      .as_array()
      .and_then(|t| t.first())
      .and_then(|t| t.as_str())
      .unwrap_or("No Title")
      .to_string();

    let doi = item["DOI"].as_str().unwrap_or("").to_string();

    // Safely extract year
    let year = item["published"]["date-parts"][0][0]
      .as_i64()
      .map(|y| y.to_string())
      .unwrap_or_else(|| "Unknown".to_string());

    // Format authors
    let author = item["author"]
      .as_array()
      .map(|authors| {
        authors
          .iter()
          .take(3)
          .filter_map(|a| {
            let family = a["family"].as_str()?;
            let given = a["given"].as_str().unwrap_or("");
            Some(format!("{} {}", given, family))
          })
          .collect::<Vec<_>>()
          .join(", ")
      })
      .unwrap_or_else(|| "Unknown Author".to_string());

    if !doi.is_empty() {
      results.push(SearchResultItem {
        title,
        author,
        year,
        doi,
      });
    }
  }

  Ok(results)
}
