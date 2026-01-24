// src/api/doi.rs
use reqwest::Client;
use std::error::Error;

pub async fn resolve_doi(doi: &str) -> Result<String, Box<dyn Error>> {
    let url = format!("https://doi.org/{}", doi);
    let client = Client::new();
    
    let resp = client
        .get(&url)
        .header("Accept", "application/x-bibtex")
        .send()
        .await?;

    if resp.status().is_success() {
        let text = resp.text().await?;
        Ok(text)
    } else {
        Err(format!("DOI lookup failed: {}", resp.status()).into())
    }
}
