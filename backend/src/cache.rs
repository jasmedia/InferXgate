use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::error::ApiResult;

#[derive(Clone)]
pub struct CacheManager {
    // ConnectionManager is already designed for concurrent use via cloning
    // No need for Arc<Mutex<>> wrapper - this was causing serialized access!
    client: Option<redis::aio::ConnectionManager>,
    ttl_seconds: u64,
    enabled: bool,
}

impl CacheManager {
    pub async fn new(redis_url: Option<String>, ttl_seconds: u64, enabled: bool) -> Self {
        if !enabled {
            debug!("Caching disabled");
            return Self {
                client: None,
                ttl_seconds,
                enabled: false,
            };
        }

        let client = if let Some(url) = redis_url {
            match Client::open(url.as_str()) {
                Ok(client) => {
                    match client.get_connection_manager().await {
                        Ok(conn) => {
                            info!("✅ Redis connection established for caching (lock-free concurrency)");
                            Some(conn)
                        }
                        Err(e) => {
                            error!("Failed to connect to Redis: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create Redis client: {}", e);
                    None
                }
            }
        } else {
            warn!("Redis URL not provided, caching disabled");
            None
        };

        Self {
            enabled: client.is_some(),
            client,
            ttl_seconds,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn get<T>(&self, key: &str) -> ApiResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        if !self.enabled {
            return Ok(None);
        }

        let client = match &self.client {
            Some(c) => c,
            None => return Ok(None),
        };

        // Clone the connection manager for concurrent access (cheap operation)
        // ConnectionManager handles connection pooling internally
        let mut conn = client.clone();
        match conn.get::<_, String>(key).await {
            Ok(value) => {
                debug!("Cache hit for key: {}", key);
                match serde_json::from_str(&value) {
                    Ok(data) => Ok(Some(data)),
                    Err(e) => {
                        error!("Failed to deserialize cached value: {}", e);
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                if e.kind() == redis::ErrorKind::TypeError {
                    debug!("Cache miss for key: {}", key);
                    Ok(None)
                } else {
                    error!("Redis error: {}", e);
                    Ok(None)
                }
            }
        }
    }

    pub async fn set<T>(&self, key: &str, value: &T) -> ApiResult<()>
    where
        T: Serialize,
    {
        if !self.enabled {
            return Ok(());
        }

        let client = match &self.client {
            Some(c) => c,
            None => return Ok(()),
        };

        let serialized = match serde_json::to_string(value) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to serialize value for caching: {}", e);
                return Ok(());
            }
        };

        // Clone the connection manager for concurrent access
        let mut conn = client.clone();
        match conn
            .set_ex::<_, _, ()>(key, serialized, self.ttl_seconds)
            .await
        {
            Ok(_) => {
                debug!(
                    "Cached value for key: {} with TTL: {}s",
                    key, self.ttl_seconds
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to cache value: {}", e);
                Ok(())
            }
        }
    }

    pub async fn delete(&self, key: &str) -> ApiResult<()> {
        if !self.enabled {
            return Ok(());
        }

        let client = match &self.client {
            Some(c) => c,
            None => return Ok(()),
        };

        // Clone the connection manager for concurrent access
        let mut conn = client.clone();
        match conn.del::<_, ()>(key).await {
            Ok(_) => {
                debug!("Deleted cache key: {}", key);
                Ok(())
            }
            Err(e) => {
                error!("Failed to delete cache key: {}", e);
                Ok(())
            }
        }
    }

    pub fn generate_cache_key(&self, model: &str, messages: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        format!("{}:{}", model, messages).hash(&mut hasher);
        format!("llm:cache:{:x}", hasher.finish())
    }
}
