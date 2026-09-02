use crate::error::{PipelineError, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    // Search settings
    pub search_provider: String,
    pub search_api_key: Option<String>,
    pub search_fallback_provider: Option<String>,
    pub search_fallback_api_key: Option<String>,
    pub search_timeout_ms: u64,
    pub max_search_results: usize,

    // Candidate processing & resilience
    pub max_concurrent_candidates: usize,
    pub candidate_timeout_ms: u64,
    pub max_retries: usize,

    // Face matching thresholds
    pub high_confidence_threshold: f32,
    pub possible_match_threshold: f32,

    // Blockchain settings
    pub chain_id: u64,
    pub rpc_primary: String,
    pub rpc_secondary: Option<String>,
    pub wallet_private_key: Option<String>,
    pub contract_address: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let search_provider = env::var("SEARCH_PROVIDER").unwrap_or_else(|_| "public_web".to_string());
        let search_api_key = env::var("SEARCH_API_KEY").ok().filter(|s| !s.trim().is_empty());
        let search_fallback_provider = env::var("SEARCH_FALLBACK_PROVIDER").ok().filter(|s| !s.trim().is_empty());
        let search_fallback_api_key = env::var("SEARCH_FALLBACK_API_KEY").ok().filter(|s| !s.trim().is_empty());

        let search_timeout_ms = env::var("SEARCH_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6000);

        let max_search_results = env::var("MAX_SEARCH_RESULTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let max_concurrent_candidates = env::var("MAX_CONCURRENT_CANDIDATES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        let candidate_timeout_ms = env::var("CANDIDATE_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8000);

        let max_retries = env::var("MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);

        let high_confidence_threshold = env::var("HIGH_CONFIDENCE_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.75);

        let possible_match_threshold = env::var("POSSIBLE_MATCH_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.55);

        let chain_id = env::var("CHAIN_ID")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(80002); // Polygon Amoy default

        let rpc_primary = env::var("RPC_PRIMARY")
            .unwrap_or_else(|_| "https://rpc-amoy.polygon.technology".to_string());

        let rpc_secondary = env::var("RPC_SECONDARY")
            .ok()
            .filter(|s| !s.trim().is_empty());

        let wallet_private_key = env::var("WALLET_PRIVATE_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty());

        let contract_address = env::var("CONTRACT_ADDRESS")
            .ok()
            .filter(|s| !s.trim().is_empty());

        if high_confidence_threshold <= possible_match_threshold {
            return Err(PipelineError::ConfigError(
                "HIGH_CONFIDENCE_THRESHOLD must be greater than POSSIBLE_MATCH_THRESHOLD".into(),
            ));
        }

        Ok(Self {
            search_provider,
            search_api_key,
            search_fallback_provider,
            search_fallback_api_key,
            search_timeout_ms,
            max_search_results,
            max_concurrent_candidates,
            candidate_timeout_ms,
            max_retries,
            high_confidence_threshold,
            possible_match_threshold,
            chain_id,
            rpc_primary,
            rpc_secondary,
            wallet_private_key,
            contract_address,
        })
    }
}
