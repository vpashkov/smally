use anyhow::Result;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use redis::{aio::ConnectionManager, AsyncCommands};
use seahash::hash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config;

pub mod lru;
use lru::LruCache;

/// Cached embedding with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEmbedding {
    pub embedding: Vec<f32>,
    pub tokens: usize,
    pub model: String,
}

pub struct EmbeddingCache {
    l1_cache: Arc<RwLock<LruCache<String, CachedEmbedding>>>,
    redis_client: ConnectionManager,
    l2_cache_ttl: u64,
}

static CACHE: OnceCell<EmbeddingCache> = OnceCell::new();

impl EmbeddingCache {
    pub async fn new() -> Result<Self> {
        let settings = config::get_settings();

        // Initialize L1 cache
        let l1_cache = Arc::new(RwLock::new(LruCache::new(settings.l1_cache_size)));

        // Connect to Redis
        let client = redis::Client::open(settings.redis_url.as_str())?;
        let redis_client = ConnectionManager::new(client).await?;

        Ok(EmbeddingCache {
            l1_cache,
            redis_client,
            l2_cache_ttl: settings.l2_cache_ttl,
        })
    }

    pub async fn get(&self, text: &str) -> Option<CachedEmbedding> {
        let cache_key = self.get_cache_key(text);

        // Check L1 cache
        {
            let cache = self.l1_cache.read();
            if let Some(cached) = cache.get(&cache_key) {
                return Some(cached.clone());
            }
        }

        // Check L2 cache (Redis)
        if let Ok(data) = self
            .redis_client
            .clone()
            .get::<_, Vec<u8>>(&cache_key)
            .await
        {
            if let Some(cached) = Self::deserialize_cached_embedding(&data) {
                // Populate L1 cache
                let mut cache = self.l1_cache.write();
                cache.put(cache_key, cached.clone());
                return Some(cached);
            }
        }

        None
    }

    pub async fn set(&self, text: &str, cached_embedding: CachedEmbedding) {
        let cache_key = self.get_cache_key(text);

        // Set in L1 cache
        {
            let mut cache = self.l1_cache.write();
            cache.put(cache_key.clone(), cached_embedding.clone());
        }

        // Set in L2 cache (async, non-blocking)
        let serialized = Self::serialize_cached_embedding(&cached_embedding);
        let ttl = self.l2_cache_ttl;
        let mut client = self.redis_client.clone();
        tokio::spawn(async move {
            let _ = client.set_ex::<_, _, ()>(&cache_key, serialized, ttl).await;
        });
    }

    #[allow(dead_code)]
    pub fn get_stats(&self) -> HashMap<String, usize> {
        let cache = self.l1_cache.read();
        let mut stats = HashMap::new();
        stats.insert("l1_size".to_string(), cache.len());
        stats.insert("l1_maxsize".to_string(), cache.capacity());
        stats
    }

    fn get_cache_key(&self, text: &str) -> String {
        let normalized = text.trim().to_lowercase();
        let hash_value = hash(normalized.as_bytes());
        format!("embed:v2:{:x}", hash_value)
    }

    fn serialize_cached_embedding(cached: &CachedEmbedding) -> Vec<u8> {
        // Use bincode for efficient serialization
        bincode::serialize(cached).unwrap_or_default()
    }

    fn deserialize_cached_embedding(data: &[u8]) -> Option<CachedEmbedding> {
        bincode::deserialize(data).ok()
    }
}

pub async fn init_cache() -> Result<()> {
    // If already initialized, return early
    if CACHE.get().is_some() {
        return Ok(());
    }

    let cache = EmbeddingCache::new().await?;
    CACHE.set(cache).ok(); // Ignore error if already set
    Ok(())
}

pub fn get_cache() -> &'static EmbeddingCache {
    CACHE.get().expect("Cache not initialized")
}
