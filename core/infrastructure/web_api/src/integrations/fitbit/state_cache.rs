use super::types::OAuthState;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// Thread-safe cache for OAuth state with automatic expiration
///
/// This cache implements "consume-on-read" semantics: once a state is retrieved
/// via `consume()`, it is immediately removed from the cache to prevent replay attacks.
#[derive(Clone)]
pub struct OAuthStateCache {
    cache: Arc<Cache<String, OAuthState>>,
}

impl OAuthStateCache {
    /// Create a new OAuth state cache with default settings
    ///
    /// Configuration:
    /// - TTL: 5 minutes (OAuth states expire quickly)
    /// - Max capacity: 10,000 entries (prevents memory exhaustion)
    pub fn new() -> Self {
        let cache = Cache::builder()
            .time_to_live(Duration::from_secs(5 * 60)) // 5 minutes
            .max_capacity(10_000) // Limit memory usage
            .build();

        Self {
            cache: Arc::new(cache),
        }
    }

    /// Insert an OAuth state into the cache
    ///
    /// The state will automatically expire after 5 minutes or when consumed.
    pub async fn insert(&self, state: String, oauth_state: OAuthState) {
        tracing::debug!(
            state = %state,
            user_id = ?oauth_state.user_id,
            "Inserting OAuth state into cache"
        );
        self.cache.insert(state, oauth_state).await;
    }

    /// Consume a state from the cache (atomic get-and-remove)
    ///
    /// This method is **atomic**: once a state is consumed, it cannot be used again.
    /// This prevents replay attacks where an attacker intercepts the callback URL
    /// and tries to use it multiple times.
    ///
    /// Returns `None` if:
    /// - The state was never inserted
    /// - The state already expired (5-minute TTL)
    /// - The state was already consumed
    pub async fn consume(&self, state: &str) -> Option<OAuthState> {
        match self.cache.remove(state).await {
            Some(oauth_state) => {
                tracing::debug!(
                    state = %state,
                    user_id = ?oauth_state.user_id,
                    age_seconds = (chrono::Utc::now() - oauth_state.created_at).num_seconds(),
                    "OAuth state consumed from cache"
                );
                Some(oauth_state)
            }
            None => {
                tracing::warn!(
                    state = %state,
                    "Attempted to consume non-existent or expired OAuth state"
                );
                None
            }
        }
    }

    /// Get the current size of the cache (for debugging/monitoring)
    ///
    /// Note: This returns an approximate count because moka processes
    /// operations asynchronously in the background.
    pub fn size(&self) -> u64 {
        self.cache.entry_count()
    }
}

impl Default for OAuthStateCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_consume() {
        let cache = OAuthStateCache::new();
        let state = "test-state-123".to_string();
        let oauth_state = OAuthState::new(
            state.clone(),
            "test-verifier".to_string(),
            Some("user1".to_string()),
        );

        cache.insert(state.clone(), oauth_state.clone()).await;

        let retrieved = cache.consume(&state).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().state, state);
    }

    #[tokio::test]
    async fn test_consume_removes_state() {
        let cache = OAuthStateCache::new();
        let state = "test-state-456".to_string();
        let oauth_state = OAuthState::new(state.clone(), "test-verifier".to_string(), None);

        cache.insert(state.clone(), oauth_state).await;

        // First consume should succeed
        let first = cache.consume(&state).await;
        assert!(first.is_some());

        // Second consume should fail (state was removed)
        let second = cache.consume(&state).await;
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn test_consume_nonexistent_state() {
        let cache = OAuthStateCache::new();

        let result = cache.consume("nonexistent-state").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_size() {
        let cache = OAuthStateCache::new();

        assert_eq!(cache.size(), 0);

        cache
            .insert(
                "state1".to_string(),
                OAuthState::new("state1".to_string(), "verifier1".to_string(), None),
            )
            .await;

        cache
            .insert(
                "state2".to_string(),
                OAuthState::new("state2".to_string(), "verifier2".to_string(), None),
            )
            .await;

        // Wait for moka to process pending operations
        // (moka processes insertions asynchronously)
        cache.cache.run_pending_tasks().await;
        assert_eq!(cache.size(), 2);

        cache.consume("state1").await;
        cache.cache.run_pending_tasks().await;
        assert_eq!(cache.size(), 1);
    }
}
