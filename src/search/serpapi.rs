use super::SearchProvider;
use async_trait::async_trait;
use crate::error::{PipelineError, Result};
use crate::models::{SearchRequest, SearchResult};
use reqwest::Client;
use serde_json::Value;

pub struct SerpApiProvider {
    api_key: String,
    client: Client,
}

impl SerpApiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::builder().build().unwrap_or_default(),
        }
    }
}

#[async_trait]
impl SearchProvider for SerpApiProvider {
    fn name(&self) -> &str {
        "serpapi"
    }

    async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchResult>> {
        let url = "https://serpapi.com/search.json";
        let response = self
            .client
            .get(url)
            .query(&[
                ("q", request.query.as_str()),
                ("api_key", self.api_key.as_str()),
                ("engine", "google"),
                ("num", &request.max_results.to_string()),
                ("hl", "en"),
                ("gl", "us"),
            ])
            .send()
            .await
            .map_err(|e| PipelineError::SearchProviderError {
                provider: self.name().into(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(PipelineError::SearchProviderError {
                provider: self.name().into(),
                message: format!("HTTP status {}", response.status()),
            });
        }

        let json: Value = response.json().await.map_err(|e| {
            PipelineError::SearchProviderError {
                provider: self.name().into(),
                message: format!("Failed to parse response JSON: {e}"),
            }
        })?;

        let mut results = Vec::new();

        // 1. Organic Search Results
        if let Some(organic_results) = json.get("organic_results").and_then(|v| v.as_array()) {
            for item in organic_results {
                if let Some(link) = item.get("link").and_then(|v| v.as_str()) {
                    let title = item.get("title").and_then(|v| v.as_str()).map(String::from);
                    let snippet = item.get("snippet").and_then(|v| v.as_str()).map(String::from);
                    let media_url = item
                        .get("thumbnail")
                        .or_else(|| item.get("image"))
                        .or_else(|| item.get("favicons").and_then(|f| f.get("high_res")))
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    results.push(SearchResult {
                        url: link.to_string(),
                        title,
                        snippet,
                        media_url,
                        provider: self.name().into(),
                    });
                }
            }
        }

        // 2. Inline Images / Knowledge Graph Images
        if let Some(inline_images) = json.get("inline_images").and_then(|v| v.as_array()) {
            for item in inline_images {
                if let Some(link) = item.get("link").or_else(|| item.get("source")).and_then(|v| v.as_str()) {
                    let title = item.get("title").and_then(|v| v.as_str()).map(String::from);
                    let media_url = item
                        .get("thumbnail")
                        .or_else(|| item.get("original"))
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    results.push(SearchResult {
                        url: link.to_string(),
                        title,
                        snippet: title.clone(),
                        media_url,
                        provider: self.name().into(),
                    });
                }
            }
        }

        if results.is_empty() {
            return Err(PipelineError::SearchProviderError {
                provider: self.name().into(),
                message: "No search results returned by SerpApi".into(),
            });
        }

        Ok(results)
    }
}
