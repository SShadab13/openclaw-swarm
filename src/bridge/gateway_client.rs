use anyhow::Result;
use reqwest;
use tracing::{info, debug, warn};

/// HTTP client for communicating with the OpenClaw gateway.
///
/// The gateway exposes a WebSocket for the Control UI, but we can
/// also check health/status via HTTP. For actual task spawning,
/// we use the bridge queue pattern (SQLite) rather than direct HTTP,
/// because the gateway doesn't expose a public task-spawning API.
pub struct GatewayClient {
    #[allow(dead_code)]
    gateway_url: String,
    #[allow(dead_code)]
    gateway_token: String,
}

impl GatewayClient {
    pub fn new(gateway_url: &str, gateway_token: &str) -> Self {
        Self {
            gateway_url: gateway_url.to_string(),
            gateway_token: gateway_token.to_string(),
        }
    }

    /// Check if the gateway is alive.
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.gateway_url);
        debug!("[GatewayClient] Health check: {}", url);

        match reqwest::get(&url).await {
            Ok(response) => {
                let healthy = response.status().is_success();
                if healthy {
                    debug!("[GatewayClient] Gateway is healthy");
                } else {
                    warn!("[GatewayClient] Gateway returned status: {}", response.status());
                }
                Ok(healthy)
            }
            Err(e) => {
                debug!("[GatewayClient] Gateway unreachable: {}", e);
                Ok(false)
            }
        }
    }

    /// Get gateway status/info.
    pub async fn get_status(&self) -> Result<Option<serde_json::Value>> {
        let url = format!("{}/status", self.gateway_url);
        
        match reqwest::get(&url).await {
            Ok(response) => {
                if response.status().is_success() {
                    let json = response.json::<serde_json::Value>().await?;
                    Ok(Some(json))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }

    /// Poll the bridge queue for pending tasks and execute them.
    ///
    /// This is the consumer side of the bridge. In practice,
    /// this would be called by an OpenClaw cron job or heartbeat.
    pub async fn poll_and_execute(&self,
        queue_db_path: &str,
    ) -> Result<()> {
        use crate::bridge::queue::BridgeQueue;

        let queue = BridgeQueue::new(queue_db_path)?;
        let pending = queue.get_pending(10)?;

        if pending.is_empty() {
            return Ok(());
        }

        info!("[GatewayClient] Found {} pending bridge tasks", pending.len());

        for task in pending {
            info!(
                "[GatewayClient] Executing bridge task {} for persona {}",
                task.id, task.persona_id
            );

            queue.mark_dispatched(&task.id)?;

            // In a real implementation, this would call the OpenClaw
            // gateway's internal API or use a local script to spawn
            // a subagent. For now, we simulate execution.
            let simulated_result = format!(
                "Simulated execution of task {} by {}\nPrompt: {} chars",
                task.task_id,
                task.persona_id,
                task.prompt.len()
            );

            queue.mark_completed(&task.id, &simulated_result)?;
        }

        Ok(())
    }
}
