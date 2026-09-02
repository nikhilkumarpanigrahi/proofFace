use crate::error::{PipelineError, Result};
use reqwest::Client;
use std::time::Duration;

pub const MAX_DOWNLOAD_SIZE_BYTES: usize = 15 * 1024 * 1024; // 15MB

pub struct ContentFetcher {
    client: Client,
    timeout: Duration,
}

impl ContentFetcher {
    pub fn new(timeout_ms: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent("ProofFace/1.0 (Verification Pipeline; +https://github.com/nikhilkumarpanigrahi/proofFace)")
            .build()
            .unwrap_or_default();

        Self {
            client,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    /// Safely fetches raw media/image bytes from a URL.
    pub async fn fetch_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| PipelineError::CandidateFetchFailed {
                url: url.to_string(),
                reason: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(PipelineError::CandidateFetchFailed {
                url: url.to_string(),
                reason: format!("HTTP status {}", response.status()),
            });
        }

        // Check Content-Length if provided
        if let Some(content_length) = response.content_length() {
            if content_length as usize > MAX_DOWNLOAD_SIZE_BYTES {
                return Err(PipelineError::CandidateFetchFailed {
                    url: url.to_string(),
                    reason: format!(
                        "Content-Length ({} bytes) exceeds limit ({} bytes)",
                        content_length, MAX_DOWNLOAD_SIZE_BYTES
                    ),
                });
            }
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| PipelineError::CandidateFetchFailed {
                url: url.to_string(),
                reason: e.to_string(),
            })?;

        if bytes.len() > MAX_DOWNLOAD_SIZE_BYTES {
            return Err(PipelineError::CandidateFetchFailed {
                url: url.to_string(),
                reason: format!("Downloaded payload exceeds size limit ({} bytes)", bytes.len()),
            });
        }

        Ok(bytes.to_vec())
    }
}
