use anyhow::Result;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use redis::{aio::ConnectionManager, AsyncCommands};
use seahash::hash;
use std::collections::HashMap;
use std::sync::Arc;

use crate::config;

pub mod lru;
use lru::LruCache;

pub struct EmbeddingCache {
    l1_cache: Arc<RwLock<LruCache<String, Vec<f32>>>>,
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

    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        let cache_key = self.get_cache_key(text);

        // Check L1 cache
        {
            let cache = self.l1_cache.read();
            if let Some(embedding) = cache.get(&cache_key) {
                return Some(embedding.clone());
            }
        }

        // Check L2 cache (Redis)
        if let Ok(data) = self
            .redis_client
            .clone()
            .get::<_, Vec<u8>>(&cache_key)
            .await
        {
            if let Some(embedding) = Self::deserialize_embedding(&data) {
                // Populate L1 cache
                let mut cache = self.l1_cache.write();
                cache.put(cache_key, embedding.clone());
                return Some(embedding);
            }
        }

        None
    }

    pub async fn set(&self, text: &str, embedding: Vec<f32>) {
        let cache_key = self.get_cache_key(text);

        // Set in L1 cache
        {
            let mut cache = self.l1_cache.write();
            cache.put(cache_key.clone(), embedding.clone());
        }

        // Set in L2 cache (async, non-blocking)
        let serialized = Self::serialize_embedding(&embedding);
        let ttl = self.l2_cache_ttl;
        let mut client = self.redis_client.clone();
        tokio::spawn(async move {
            let _ = client
                .set_ex::<_, _, ()>(&cache_key, serialized, ttl)
                .await;
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

    fn serialize_embedding(embedding: &[f32]) -> Vec<u8> {
        embedding
            .iter()
            .flat_map(|&f| f.to_le_bytes())
            .collect()
    }

    fn deserialize_embedding(data: &[u8]) -> Option<Vec<f32>> {
        if data.is_empty() || data.len() % 4 != 0 {
            return None;
        }

        let mut embedding = Vec::with_capacity(data.len() / 4);
        for chunk in data.chunks_exact(4) {
            let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
            embedding.push(f32::from_le_bytes(bytes));
        }

        Some(embedding)
    }
}

pub async fn init_cache() -> Result<()> {
    let cache = EmbeddingCache::new().await?;
    CACHE
        .set(cache)
        .map_err(|_| anyhow::anyhow!("Cache already initialized"))?;
    Ok(())
}

pub fn get_cache() -> &'static EmbeddingCache {
    CACHE.get().expect("Cache not initialized")
}
