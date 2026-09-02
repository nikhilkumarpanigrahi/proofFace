pub mod brave;
pub mod orchestrator;
pub mod public_web;
pub mod serpapi;
pub mod tavily;

use async_trait::async_trait;
use crate::error::Result;
use crate::models::{SearchRequest, SearchResult};

#[async_trait]
pub trait SearchProvider: Send + Sync {
    /// Human-readable provider name.
    fn name(&self) -> &str;

    /// Executes a genuine external search and returns parsed candidates.
    async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchResult>>;
}
