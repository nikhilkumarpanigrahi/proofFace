use super::SearchProvider;
use async_trait::async_trait;
use crate::error::{PipelineError, Result};
use crate::models::{SearchRequest, SearchResult};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub struct SerpApiProvider {
    api_key: String,
    client: Client,
}

impl SerpApiProvider {
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("ProofFace-Engine/1.0 (Visual Verification Bot)")
            .build()
            .unwrap_or_default();

        Self { api_key, client }
    }

    /// Uploads raw image bytes to a high-speed temporary buffer to get a public URL for Google Lens.
    async fn upload_temporary_image(&self, bytes: &[u8]) -> Option<String> {
        // Attempt 1: catbox.moe
        let part = Part::bytes(bytes.to_vec())
            .file_name("face.jpg")
            .mime_str("image/jpeg")
            .ok()?;

        let form = Form::new()
            .text("reqtype", "fileupload")
            .part("fileToUpload", part);

        if let Ok(resp) = self.client.post("https://catbox.moe/user/api.php").multipart(form).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    let trimmed = text.trim();
                    if trimmed.starts_with("http") {
                        return Some(trimmed.to_string());
                    }
                }
            }
        }

        // Attempt 2: tmpfiles.org fallback
        let part2 = Part::bytes(bytes.to_vec())
            .file_name("face.jpg")
            .mime_str("image/jpeg")
            .ok()?;

        let form2 = Form::new().part("file", part2);

        if let Ok(resp) = self.client.post("https://tmpfiles.org/api/v1/upload").multipart(form2).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<Value>().await {
                    if let Some(url_str) = json.get("data").and_then(|d| d.get("url")).and_then(|v| v.as_str()) {
                        let dl_url = url_str.replace("tmpfiles.org/", "tmpfiles.org/dl/");
                        return Some(dl_url);
                    }
                }
            }
        }

        None
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

        // 1. PRIMARY: Visual Reverse Image Search via Google Lens (Zero Text Dependency)
        if let Some(img_bytes) = &request.image_bytes {
            if let Some(public_image_url) = self.upload_temporary_image(img_bytes).await {
                let lens_response = self
                    .client
                    .get(url)
                    .query(&[
                        ("engine", "google_lens"),
                        ("url", public_image_url.as_str()),
                        ("api_key", self.api_key.as_str()),
                        ("hl", "en"),
                    ])
                    .send()
                    .await;

                if let Ok(resp) = lens_response {
                    if resp.status().is_success() {
                        if let Ok(json) = resp.json::<Value>().await {
                            if let Some(visual_matches) = json.get("visual_matches").and_then(|v| v.as_array()) {
                                for item in visual_matches.iter().take(request.max_results) {
                                    let link = item
                                        .get("link")
                                        .or_else(|| item.get("source"))
                                        .and_then(|v| v.as_str());

                                    let media_url = item
                                        .get("thumbnail")
                                        .or_else(|| item.get("image"))
                                        .and_then(|v| v.as_str());

                                    if let (Some(l), Some(m)) = (link, media_url) {
                                        let title = item.get("title").and_then(|v| v.as_str()).map(String::from);
                                        let snippet = item.get("source").and_then(|v| v.as_str()).map(String::from);

                                        results.push(SearchResult {
                                            url: l.to_string(),
                                            title,
                                            snippet,
                                            media_url: Some(m.to_string()),
                                            provider: "serpapi_google_lens".into(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. SECONDARY: Google Images Search
        if results.len() < request.max_results {
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
                            for item in images_arr.iter().take(request.max_results - results.len()) {
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
        }

        if results.is_empty() {
            return Err(PipelineError::SearchProviderError {
                provider: self.name().into(),
                message: format!("No candidate matches found on SerpApi for query '{}'", request.query),
            });
        }

        Ok(results)
    }
}
