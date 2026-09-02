use super::SearchProvider;
use async_trait::async_trait;
use crate::error::{PipelineError, Result};
use crate::models::{SearchRequest, SearchResult};
use reqwest::Client;
use serde_json::Value;

pub struct BraveSearchProvider {
    api_key: String,
    client: Client,
}

impl BraveSearchProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::builder().build().unwrap_or_default(),
        }
    }
}

#[async_trait]
impl SearchProvider for BraveSearchProvider {
    fn name(&self) -> &str {
        "brave"
    }

    async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchResult>> {
        let url = "https://api.search.brave.com/res/v1/web/search";
        let response = self
            .client
            .get(url)
            .header("X-Subscription-Token", &self.api_key)
            .query(&[
                ("q", request.query.as_str()),
                ("count", &request.max_results.to_string()),
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

        if let Some(web_results) = json
            .get("web")
            .and_then(|w| w.get("results"))
            .and_then(|r| r.as_array())
        {
            for item in web_results {
                if let Some(link) = item.get("url").and_then(|v| v.as_str()) {
                    let title = item.get("title").and_then(|v| v.as_str()).map(String::from);
                    let snippet = item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let media_url = item
                        .get("thumbnail")
                        .and_then(|t| t.get("src"))
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

        Ok(results)
    }
}
