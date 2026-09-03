use super::{
    brave::BraveSearchProvider, public_web::PublicWebSearchProvider, serpapi::SerpApiProvider,
    tavily::TavilySearchProvider, SearchProvider,
};
use crate::config::Config;
use crate::content::dedup::CandidateDeduplicator;
use crate::error::{PipelineError, Result};
use crate::models::{SearchRequest, SearchResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

pub struct SearchOrchestrator {
    primary: Arc<dyn SearchProvider>,
    fallback: Option<Arc<dyn SearchProvider>>,
    timeout_duration: Duration,
    max_retries: usize,
}

impl SearchOrchestrator {
    pub fn new(
        primary: Arc<dyn SearchProvider>,
        fallback: Option<Arc<dyn SearchProvider>>,
        timeout_ms: u64,
        max_retries: usize,
    ) -> Self {
        Self {
            primary,
            fallback,
            timeout_duration: Duration::from_millis(timeout_ms),
            max_retries,
        }
    }

    pub fn from_config(config: &Config) -> Self {
        let primary =
            Self::create_provider(&config.search_provider, config.search_api_key.as_deref());

        let fallback = if let Some(fb_name) = &config.search_fallback_provider {
            Some(Self::create_provider(
                fb_name,
                config.search_fallback_api_key.as_deref(),
            ))
        } else if config.search_provider != "public_web" {
            Some(Arc::new(PublicWebSearchProvider::new()) as Arc<dyn SearchProvider>)
        } else {
            None
        };

        Self::new(
            primary,
            fallback,
            config.search_timeout_ms,
            config.max_retries,
        )
    }

    fn create_provider(provider_name: &str, api_key: Option<&str>) -> Arc<dyn SearchProvider> {
        match provider_name.to_lowercase().as_str() {
            "serpapi" => {
                if let Some(key) = api_key {
                    Arc::new(SerpApiProvider::new(key.to_string()))
                } else {
                    warn!("SerpApi API key missing, falling back to public web provider");
                    Arc::new(PublicWebSearchProvider::new())
                }
            }
            "brave" => {
                if let Some(key) = api_key {
                    Arc::new(BraveSearchProvider::new(key.to_string()))
                } else {
                    warn!("Brave API key missing, falling back to public web provider");
                    Arc::new(PublicWebSearchProvider::new())
                }
            }
            "tavily" => {
                if let Some(key) = api_key {
                    Arc::new(TavilySearchProvider::new(key.to_string()))
                } else {
                    warn!("Tavily API key missing, falling back to public web provider");
                    Arc::new(PublicWebSearchProvider::new())
                }
            }
            _ => Arc::new(PublicWebSearchProvider::new()),
        }
    }

    /// Executes search with retries and automatic fallback to secondary provider.
    pub async fn search_with_resilience(
        &self,
        request: &SearchRequest,
    ) -> Result<Vec<SearchResult>> {
        // Try Primary Provider with retries
        info!(provider = self.primary.name(), query = %request.query, "Initiating web search discovery");
        match self.try_provider(self.primary.as_ref(), request).await {
            Ok(results) if !results.is_empty() => {
                info!(
                    provider = self.primary.name(),
                    count = results.len(),
                    "Search successful on primary provider"
                );
                return Ok(Self::deduplicate_results(results));
            }
            Ok(_) => {
                warn!(
                    provider = self.primary.name(),
                    "Primary search returned 0 candidates"
                );
            }
            Err(e) => {
                warn!(provider = self.primary.name(), error = %e, "Primary search provider failed");
            }
        }

        // Try Fallback Provider if configured
        if let Some(fallback_provider) = &self.fallback {
            info!(
                provider = fallback_provider.name(),
                "Switching to search fallback provider"
            );
            match self.try_provider(fallback_provider.as_ref(), request).await {
                Ok(results) if !results.is_empty() => {
                    info!(
                        provider = fallback_provider.name(),
                        count = results.len(),
                        "Search successful on fallback provider"
                    );
                    return Ok(Self::deduplicate_results(results));
                }
                Ok(_) => {
                    warn!(
                        provider = fallback_provider.name(),
                        "Fallback search returned 0 candidates"
                    );
                }
                Err(e) => {
                    warn!(provider = fallback_provider.name(), error = %e, "Fallback search provider failed");
                }
            }
        }

        Err(PipelineError::SearchExhausted)
    }

    async fn try_provider(
        &self,
        provider: &dyn SearchProvider,
        request: &SearchRequest,
    ) -> Result<Vec<SearchResult>> {
        let mut attempts = 0;
        let mut backoff = Duration::from_millis(300);

        while attempts <= self.max_retries {
            attempts += 1;
            match timeout(self.timeout_duration, provider.search(request)).await {
                Ok(Ok(results)) => return Ok(results),
                Ok(Err(e)) => {
                    if attempts > self.max_retries {
                        return Err(e);
                    }
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
                Err(_) => {
                    if attempts > self.max_retries {
                        return Err(PipelineError::SearchTimeout(
                            self.timeout_duration.as_millis() as u64,
                        ));
                    }
                    tokio::time::sleep(backoff).await;
                    backoff *= 2;
                }
            }
        }

        Err(PipelineError::SearchExhausted)
    }

    fn deduplicate_results(results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut dedup = CandidateDeduplicator::new();
        let mut unique = Vec::new();

        for item in results {
            if dedup.check_and_insert(&item.url) {
                unique.push(item);
            }
        }

        unique
    }
}
