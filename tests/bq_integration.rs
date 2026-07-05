// OpenClaw Swarm — BigQuery Adapter Integration Tests
//
// Unit test: authenticate() fails fast with bad credentials.
// Integration test: lists datasets in bigquery-public-data when credentials are present.

use openclaw_swarm::adapters::bq_adapter::{BqAdapterLive, BqConfig, BigQueryAdapter};

#[tokio::test]
async fn test_authenticate_fails_without_credentials() {
    let config = BqConfig {
        project_id: "test-project".to_string(),
        credentials_path: "/nonexistent/path.json".to_string(),
        ..Default::default()
    };
    let mut adapter = BqAdapterLive::new();
    let result = adapter.authenticate(&config).await;
    assert!(result.is_err(), "Should fail with nonexistent credentials file");
}

#[tokio::test]
async fn test_list_bigquery_public_datasets() {
    let creds = match std::env::var("BQ_CREDENTIALS_PATH") {
        Ok(p) => p,
        Err(_) => {
            println!("SKIP: BQ_CREDENTIALS_PATH not set");
            return;
        }
    };
    let project =
        std::env::var("BQ_PROJECT_ID").unwrap_or_else(|_| "bigquery-public-data".to_string());

    let config = BqConfig {
        project_id: project,
        credentials_path: creds,
        ..Default::default()
    };

    let mut adapter = BqAdapterLive::new();
    adapter.authenticate(&config).await.expect("authenticate() failed");

    let datasets = adapter.list_datasets().await.expect("list_datasets() failed");
    println!("Found {} datasets", datasets.len());
    for ds in datasets.iter().take(5) {
        println!("  dataset: {} ({})", ds.dataset_id, ds.location);
    }
    assert!(!datasets.is_empty(), "Expected at least one dataset");
}
