//! Browser pool — pre-launched Chromium instances for fast session creation.
//!
//! Maintains a pool of idle [`CdpBackend`] instances. When a session is
//! created, it acquires from the pool instead of cold-launching Chromium
//! (2-5s savings). When a session is closed, the backend is returned to
//! the pool for reuse.
//!
//! A background warmer task monitors the pool level and pre-launches new
//! backends when the idle count drops below `min_idle`.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use ans_cdp::CdpBackend;
use ans_core::backend::BrowserBackend;
use ans_stealth::StealthConfig;
use tokio::sync::Mutex;
use tracing;

/// A pool of pre-launched Chromium backends.
///
/// Cheap to clone — wraps an `Arc<Mutex<VecDeque<CdpBackend>>>`.
#[derive(Clone)]
pub struct BrowserPool {
    idle: Arc<Mutex<VecDeque<CdpBackend>>>,
    max_size: usize,
    min_idle: usize,
}

impl BrowserPool {
    /// Create a new browser pool.
    ///
    /// `max_size` caps the total idle backends. The warmer maintains
    /// at least `min_idle` backends (clamped to `max_size`).
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        let min_idle = (max_size / 2).max(1);
        Self {
            idle: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
            min_idle,
        }
    }

    /// Create a pool with explicit min/max bounds.
    #[must_use]
    pub fn with_bounds(max_size: usize, min_idle: usize) -> Self {
        Self {
            idle: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
            min_idle: min_idle.min(max_size),
        }
    }

    /// Acquire a backend: pop from pool if available, else launch fresh.
    ///
    /// This is the hot path — called on every `SessionManager::create()`.
    pub async fn acquire(&self) -> Result<CdpBackend, ans_core::backend::BrowserError> {
        // Fast path: pop from idle pool
        if let Some(backend) = self.idle.lock().await.pop_front() {
            let idle_count = self.idle_bytes().await;
            tracing::debug!(pool_idle = idle_count, "Acquired backend from pool");
            return Ok(backend);
        }

        // Slow path: cold launch
        tracing::info!("Pool empty, cold-launching Chromium");
        CdpBackend::launch(Some(StealthConfig::standard())).await
    }

    /// Release a backend back to the pool.
    ///
    /// If the pool is at capacity, the backend is closed instead.
    /// Called on every `SessionManager::close()`.
    pub async fn release(&self, backend: CdpBackend) {
        let mut idle = self.idle.lock().await;
        if idle.len() < self.max_size {
            idle.push_back(backend);
            tracing::debug!(pool_idle = idle.len(), "Returned backend to pool");
        } else {
            drop(idle);
            tracing::debug!("Pool full, closing backend");
            let _ = backend.close().await;
        }
    }

    /// Shut down the pool, closing all idle backends.
    pub async fn shutdown(&self) {
        let backends: Vec<CdpBackend> = self.idle.lock().await.drain(..).collect();
        tracing::info!(count = backends.len(), "Shutting down browser pool");
        for backend in backends {
            let _ = backend.close().await;
        }
    }

    /// Number of idle backends currently in the pool.
    #[must_use]
    pub async fn idle_count(&self) -> usize {
        self.idle.lock().await.len()
    }

    /// Background task: maintain `min_idle` backends in the pool.
    ///
    /// Runs indefinitely — `tokio::spawn` this at daemon startup.
    /// The task exits when all clones of `BrowserPool` are dropped
    /// (the `Arc` refcount drops to zero and `Mutex` is no longer
    ///  held by any holder).
    pub async fn warmer_task(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            let current = self.idle_count().await;
            if current < self.min_idle {
                let needed = self.min_idle - current;
                tracing::debug!(current, needed, "Warmer: pre-launching backends");
                for _ in 0..needed {
                    match CdpBackend::launch(Some(StealthConfig::standard())).await {
                        Ok(backend) => {
                            let mut idle = self.idle.lock().await;
                            if idle.len() < self.max_size {
                                idle.push_back(backend);
                            } else {
                                drop(idle);
                                let _ = backend.close().await;
                                break; // pool is full, stop launching
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Warmer: failed to pre-launch backend");
                            // Don't break — transient errors happen, retry next tick
                        }
                    }
                }
            }
        }
    }

    /// Launch the warmer as a spawned background task.
    ///
    /// Returns the `JoinHandle` so callers can abort on shutdown.
    pub fn spawn_warmer(&self) -> tokio::task::JoinHandle<()> {
        let pool = self.clone();
        tokio::spawn(async move { pool.warmer_task().await })
    }

    /// Approximate idle byte footprint (for debug logging).
    async fn idle_bytes(&self) -> usize {
        self.idle.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_starts_empty() {
        let pool = BrowserPool::new(4);
        assert_eq!(pool.idle_count().await, 0);
    }

    #[tokio::test]
    async fn test_pool_with_bounds() {
        let pool = BrowserPool::with_bounds(8, 3);
        assert_eq!(pool.max_size, 8);
        assert_eq!(pool.min_idle, 3);
    }

    #[tokio::test]
    async fn test_min_idle_clamped_to_max() {
        let pool = BrowserPool::with_bounds(2, 10);
        assert_eq!(pool.min_idle, 2); // clamped to max_size
    }

    #[tokio::test]
    async fn test_shutdown_clears_pool() {
        let pool = BrowserPool::new(4);
        pool.shutdown().await;
        assert_eq!(pool.idle_count().await, 0);
    }
}
