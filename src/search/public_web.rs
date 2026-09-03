use super::SearchProvider;
use crate::error::{PipelineError, Result};
use crate::models::{SearchRequest, SearchResult};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

pub struct PublicWebSearchProvider {
    client: Client,
}

impl PublicWebSearchProvider {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("ProofFace-PublicDiscovery/1.0 (Public Research Pipeline; contact: https://github.com/nikhilkumarpanigrahi/proofFace)")
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for PublicWebSearchProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SearchProvider for PublicWebSearchProvider {
    fn name(&self) -> &str {
        "public_web"
    }

    async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        // 1. Query Wikipedia / Wikimedia API for real public pages and images matching query
        let wiki_url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&format=json&utf8=1&srlimit={}",
            urlencoding::encode(&request.query),
            request.max_results
        );

        if let Ok(resp) = self.client.get(&wiki_url).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(search_items) = json
                        .get("query")
                        .and_then(|q| q.get("search"))
                        .and_then(|s| s.as_array())
                    {
                        for item in search_items {
                            if let Some(title) = item.get("title").and_then(|t| t.as_str()) {
                                let snippet =
                                    item.get("snippet").and_then(|s| s.as_str()).map(|s| {
                                        s.replace("<span class=\"searchmatch\">", "")
                                            .replace("</span>", "")
                                    });

                                let page_url = format!(
                                    "https://en.wikipedia.org/wiki/{}",
                                    urlencoding::encode(title)
                                );

                                // Fetch page thumbnail image via Wikipedia summary API
                                let summary_url = format!(
                                    "https://en.wikipedia.org/api/rest_v1/page/summary/{}",
                                    urlencoding::encode(title)
                                );

                                let mut media_url = None;
                                if let Ok(summary_resp) = self.client.get(&summary_url).send().await
                                {
                                    if let Ok(summary_json) = summary_resp.json::<Value>().await {
                                        if let Some(src) = summary_json
                                            .get("thumbnail")
                                            .and_then(|t| t.get("source"))
                                            .and_then(|s| s.as_str())
                                        {
                                            media_url = Some(src.to_string());
                                        }
                                    }
                                }

                                results.push(SearchResult {
                                    url: page_url,
                                    title: Some(title.to_string()),
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

        // 2. Query DuckDuckGo Instant Answers API
        let ddg_url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            urlencoding::encode(&request.query)
        );

        if let Ok(resp) = self.client.get(&ddg_url).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Value>().await {
                    let abstract_url = json
                        .get("AbstractURL")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let heading = json.get("Heading").and_then(|v| v.as_str()).unwrap_or("");
                    let abstract_text = json
                        .get("AbstractText")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let image_url = json
                        .get("Image")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty());

                    if !abstract_url.is_empty() {
                        results.push(SearchResult {
                            url: abstract_url.to_string(),
                            title: Some(heading.to_string()),
                            snippet: Some(abstract_text.to_string()),
                            media_url: image_url.map(|s| {
                                if s.starts_with("http") {
                                    s.to_string()
                                } else {
                                    format!("https://duckduckgo.com{s}")
                                }
                            }),
                            provider: self.name().into(),
                        });
                    }

                    // Related topics
                    if let Some(related) = json.get("RelatedTopics").and_then(|r| r.as_array()) {
                        for topic in related.iter().take(5) {
                            if let Some(first_url) = topic.get("FirstURL").and_then(|v| v.as_str())
                            {
                                let text =
                                    topic.get("Text").and_then(|v| v.as_str()).map(String::from);
                                let icon_url = topic
                                    .get("Icon")
                                    .and_then(|i| i.get("URL"))
                                    .and_then(|u| u.as_str())
                                    .filter(|s| !s.is_empty())
                                    .map(|s| {
                                        if s.starts_with("http") {
                                            s.to_string()
                                        } else {
                                            format!("https://duckduckgo.com{s}")
                                        }
                                    });

                                results.push(SearchResult {
                                    url: first_url.to_string(),
                                    title: text.clone(),
                                    snippet: text,
                                    media_url: icon_url,
                                    provider: self.name().into(),
                                });
                            }
                        }
                    }
                }
            }
        }

        if results.is_empty() {
            return Err(PipelineError::SearchProviderError {
                provider: self.name().into(),
                message: "No public search candidates found for query".into(),
            });
        }

        Ok(results)
    }
}
