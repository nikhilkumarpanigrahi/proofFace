pub mod retry;

use std::sync::Arc;
use tokio::sync::Semaphore;

/// Bounded concurrency manager for candidate processing.
pub struct BoundedPool {
    semaphore: Arc<Semaphore>,
}

impl BoundedPool {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
        }
    }

    pub fn semaphore(&self) -> Arc<Semaphore> {
        Arc::clone(&self.semaphore)
    }
}
