use std::future::Future;
use std::time::Duration;

/// Executes an async operation with exponential backoff retries.
pub async fn retry_with_backoff<F, Fut, T, E>(
    max_retries: usize,
    initial_delay: Duration,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempts = 0;
    let mut delay = initial_delay;

    loop {
        attempts += 1;
        match operation().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempts > max_retries {
                    return Err(e);
                }
                tokio::time::sleep(delay).await;
                delay = delay.saturating_mul(2);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_retry_succeeds_on_second_attempt() {
        let counter = AtomicUsize::new(0);

        let result: Result<i32, &str> =
            retry_with_backoff(3, Duration::from_millis(10), || async {
                let val = counter.fetch_add(1, Ordering::SeqCst);
                if val == 0 {
                    Err("temporary failure")
                } else {
                    Ok(42)
                }
            })
            .await;

        assert_eq!(result, Ok(42));
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
