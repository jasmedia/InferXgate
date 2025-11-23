use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastLatency,
    LeastCost,
    Random,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderHealth {
    pub provider: String,
    pub model: String,
    pub success_count: u64,
    pub error_count: u64,
    pub total_latency_ms: u64,
    pub last_error_time: Option<i64>,
    pub available: bool,
}

impl ProviderHealth {
    pub fn new(provider: String, model: String) -> Self {
        Self {
            provider,
            model,
            success_count: 0,
            error_count: 0,
            total_latency_ms: 0,
            last_error_time: None,
            available: true,
        }
    }

    pub fn average_latency_ms(&self) -> u64 {
        if self.success_count == 0 {
            0
        } else {
            self.total_latency_ms / self.success_count
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.error_count;
        if total == 0 {
            1.0
        } else {
            self.success_count as f64 / total as f64
        }
    }
}

pub struct LoadBalancer {
    strategy: LoadBalancingStrategy,
    provider_health: Arc<RwLock<HashMap<String, ProviderHealth>>>,
    round_robin_index: Arc<RwLock<HashMap<String, usize>>>,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            strategy,
            provider_health: Arc::new(RwLock::new(HashMap::new())),
            round_robin_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn select_provider(
        &self,
        model: &str,
        available_providers: &[(String, String, f64)], // (provider, target_model, cost_per_1k)
    ) -> Option<String> {
        if available_providers.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                self.select_round_robin(model, available_providers).await
            }
            LoadBalancingStrategy::LeastLatency => {
                self.select_least_latency(available_providers).await
            }
            LoadBalancingStrategy::LeastCost => self.select_least_cost(available_providers).await,
            LoadBalancingStrategy::Random => self.select_random(available_providers).await,
        }
    }

    async fn select_round_robin(
        &self,
        model: &str,
        providers: &[(String, String, f64)],
    ) -> Option<String> {
        let mut index_map = self.round_robin_index.write().await;
        let current_index = index_map.entry(model.to_string()).or_insert(0);

        let selected = providers.get(*current_index % providers.len())?;
        *current_index = (*current_index + 1) % providers.len();

        debug!(
            "Round-robin selected provider: {} for model: {}",
            selected.0, model
        );
        Some(selected.0.clone())
    }

    async fn select_least_latency(&self, providers: &[(String, String, f64)]) -> Option<String> {
        let health_map = self.provider_health.read().await;

        let mut best_provider = providers.first()?.0.clone();
        let mut best_latency = u64::MAX;

        for (provider, model, _) in providers {
            let key = format!("{}:{}", provider, model);
            if let Some(health) = health_map.get(&key) {
                if health.available && health.success_count > 0 {
                    let avg_latency = health.average_latency_ms();
                    if avg_latency < best_latency {
                        best_latency = avg_latency;
                        best_provider = provider.clone();
                    }
                }
            }
        }

        debug!(
            "Least-latency selected provider: {} (avg: {}ms)",
            best_provider, best_latency
        );
        Some(best_provider)
    }

    async fn select_least_cost(&self, providers: &[(String, String, f64)]) -> Option<String> {
        let mut best_provider = providers.first()?.0.clone();
        let mut best_cost = f64::MAX;

        for (provider, _, cost) in providers {
            if *cost < best_cost {
                best_cost = *cost;
                best_provider = provider.clone();
            }
        }

        debug!(
            "Least-cost selected provider: {} (cost: ${:.4}/1k tokens)",
            best_provider, best_cost
        );
        Some(best_provider)
    }

    async fn select_random(&self, providers: &[(String, String, f64)]) -> Option<String> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..providers.len());
        let selected = providers.get(index)?;

        debug!("Random selected provider: {}", selected.0);
        Some(selected.0.clone())
    }

    pub async fn record_success(&self, provider: &str, model: &str, latency_ms: u64) {
        let key = format!("{}:{}", provider, model);
        let mut health_map = self.provider_health.write().await;

        let health = health_map
            .entry(key)
            .or_insert_with(|| ProviderHealth::new(provider.to_string(), model.to_string()));

        health.success_count += 1;
        health.total_latency_ms += latency_ms;
        health.available = true;

        debug!(
            "Recorded success for {}:{} - avg latency: {}ms, success rate: {:.2}%",
            provider,
            model,
            health.average_latency_ms(),
            health.success_rate() * 100.0
        );
    }

    pub async fn record_error(&self, provider: &str, model: &str) {
        let key = format!("{}:{}", provider, model);
        let mut health_map = self.provider_health.write().await;

        let health = health_map
            .entry(key)
            .or_insert_with(|| ProviderHealth::new(provider.to_string(), model.to_string()));

        health.error_count += 1;
        health.last_error_time = Some(chrono::Utc::now().timestamp());

        // Mark as unavailable if error rate is too high
        if health.success_rate() < 0.5 && health.error_count > 3 {
            health.available = false;
            debug!(
                "Marked {}:{} as unavailable due to high error rate",
                provider, model
            );
        }

        debug!(
            "Recorded error for {}:{} - success rate: {:.2}%",
            provider,
            model,
            health.success_rate() * 100.0
        );
    }

    pub async fn get_provider_health(&self, provider: &str, model: &str) -> Option<ProviderHealth> {
        let key = format!("{}:{}", provider, model);
        let health_map = self.provider_health.read().await;
        health_map.get(&key).cloned()
    }

    pub async fn get_all_health_stats(&self) -> Vec<ProviderHealth> {
        let health_map = self.provider_health.read().await;
        health_map.values().cloned().collect()
    }

    pub async fn reset_provider(&self, provider: &str, model: &str) {
        let key = format!("{}:{}", provider, model);
        let mut health_map = self.provider_health.write().await;

        if let Some(health) = health_map.get_mut(&key) {
            health.available = true;
            health.error_count = 0;
            health.last_error_time = None;
            debug!("Reset health stats for {}:{}", provider, model);
        }
    }
}
