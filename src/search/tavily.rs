use super::SearchProvider;
use crate::error::{PipelineError, Result};
use crate::models::{SearchRequest, SearchResult};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

pub struct TavilySearchProvider {
    api_key: String,
    client: Client,
}

impl TavilySearchProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::builder().build().unwrap_or_default(),
        }
    }
}

#[async_trait]
impl SearchProvider for TavilySearchProvider {
    fn name(&self) -> &str {
        "tavily"
    }

    async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchResult>> {
        let url = "https://api.tavily.com/search";
        let body = json!({
            "api_key": self.api_key,
            "query": request.query,
            "max_results": request.max_results,
            "include_images": true
        });

        let response = self
            .client
            .post(url)
            .json(&body)
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

        let json: Value =
            response
                .json()
                .await
                .map_err(|e| PipelineError::SearchProviderError {
                    provider: self.name().into(),
                    message: format!("Failed to parse response JSON: {e}"),
                })?;

        let mut results = Vec::new();

        if let Some(res_arr) = json.get("results").and_then(|v| v.as_array()) {
            for item in res_arr {
                if let Some(link) = item.get("url").and_then(|v| v.as_str()) {
                    let title = item.get("title").and_then(|v| v.as_str()).map(String::from);
                    let snippet = item
                        .get("content")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let media_url = item
                        .get("images")
                        .and_then(|img_arr| img_arr.as_array())
                        .and_then(|arr| arr.first())
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
