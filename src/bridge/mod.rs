pub mod queue;
pub mod gateway_client;

/// The OpenClaw Bridge connects the swarm to the OpenClaw gateway.
///
/// Pattern: SQLite queue tables (durable, observable, zero extra deps)
///
/// Flow:
/// 1. Swarm writes task request to bridge_queue table
/// 2. OpenClaw (or a bridge poller) reads pending tasks
/// 3. OpenClaw executes via sessions_spawn
/// 4. Results written to bridge_results table
/// 5. Swarm reads results, continues execution
///
/// This is decoupled: either side can crash and resume.
pub struct BridgeConfig {
    pub db_path: String,
    pub gateway_url: String,
    pub gateway_token: String,
    pub poll_interval_secs: u64,
}

impl BridgeConfig {
    pub fn default_with_db(db_path: &str) -> Self {
        Self {
            db_path: db_path.to_string(),
            gateway_url: std::env::var("OPENCLAW_GATEWAY_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:18679".to_string()),
            gateway_token: std::env::var("OPENCLAW_TOKEN")
                .unwrap_or_else(|_| "0124d10567d41ba2afbc196450fcc612".to_string()),
            poll_interval_secs: 30,
        }
    }
}
