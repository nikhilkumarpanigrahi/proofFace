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
        let mut results = Vec::new();

        // 1. First query Google Images Engine (engine: google_images) for direct high-resolution candidate posts
        let img_response = self
            .client
            .get(url)
            .query(&[
                ("q", request.query.as_str()),
                ("api_key", self.api_key.as_str()),
                ("engine", "google_images"),
                ("num", &request.max_results.to_string()),
                ("hl", "en"),
                ("gl", "us"),
            ])
            .send()
            .await;

        if let Ok(resp) = img_response {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(images_arr) = json.get("images_results").and_then(|v| v.as_array()) {
                        for item in images_arr.iter().take(request.max_results) {
                            let link = item
                                .get("link")
                                .or_else(|| item.get("source"))
                                .and_then(|v| v.as_str());

                            let media_url = item
                                .get("thumbnail")
                                .or_else(|| item.get("original"))
                                .and_then(|v| v.as_str());

                            if let (Some(l), Some(m)) = (link, media_url) {
                                let title = item.get("title").and_then(|v| v.as_str()).map(String::from);
                                let snippet = item.get("snippet").and_then(|v| v.as_str()).map(String::from);

                                results.push(SearchResult {
                                    url: l.to_string(),
                                    title,
                                    snippet,
                                    media_url: Some(m.to_string()),
                                    provider: self.name().into(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // 2. If Google Images returned fewer than max_results, query standard Google Web Search
        if results.len() < request.max_results {
            let web_response = self
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
                .await;

            if let Ok(resp) = web_response {
                if resp.status().is_success() {
                    if let Ok(json) = resp.json::<Value>().await {
                        if let Some(organic_results) = json.get("organic_results").and_then(|v| v.as_array()) {
                            for item in organic_results {
                                if let Some(link) = item.get("link").and_then(|v| v.as_str()) {
                                    let title = item.get("title").and_then(|v| v.as_str()).map(String::from);
                                    let snippet = item.get("snippet").and_then(|v| v.as_str()).map(String::from);
                                    let media_url = item
                                        .get("thumbnail")
                                        .or_else(|| item.get("image"))
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
                    }
                }
            }
        }

        if results.is_empty() {
            return Err(PipelineError::SearchProviderError {
                provider: self.name().into(),
                message: format!("No candidate images found on SerpApi for query '{}'", request.query),
            });
        }

        Ok(results)
    }
}
