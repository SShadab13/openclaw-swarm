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

#[tokio::test]
async fn test_get_schema_public_table() {
    let creds = match std::env::var("BQ_CREDENTIALS_PATH") {
        Ok(p) => p,
        Err(_) => {
            println!("SKIP: BQ_CREDENTIALS_PATH not set");
            return;
        }
    };
    let project = std::env::var("BQ_PROJECT_ID").expect("BQ_PROJECT_ID must be set with creds");

    let config = BqConfig {
        project_id: project,
        credentials_path: creds,
        ..Default::default()
    };

    let mut adapter = BqAdapterLive::new();
    adapter.authenticate(&config).await.expect("authenticate() failed");

    let schema = adapter
        .get_schema("bigquery-public-data.austin_311.311_service_requests")
        .await
        .expect("get_schema() failed");

    println!("Table {} has {} columns", schema.table_id, schema.columns.len());
    assert_eq!(schema.dataset_id, "austin_311");
    assert!(!schema.columns.is_empty(), "Expected at least one column");
}

// The two tests below use ADC (empty credentials_path) and gate on
// BQ_PROJECT_ID because jobs.query needs job-creation rights in a project
// you own. Queries stay inside BigQuery's free tier.

#[tokio::test]
async fn test_run_query_guard_blocks_expensive_scan() {
    let project = match std::env::var("BQ_PROJECT_ID") {
        Ok(p) => p,
        Err(_) => {
            println!("SKIP: BQ_PROJECT_ID not set");
            return;
        }
    };
    let config = BqConfig {
        project_id: project,
        credentials_path: String::new(), // ADC
        max_bytes_scanned: Some(1),      // absurdly low: everything must trip
        ..Default::default()
    };
    let mut adapter = BqAdapterLive::new();
    adapter.authenticate(&config).await.expect("authenticate() failed");

    let result = adapter
        .run_query("SELECT unique_key FROM `bigquery-public-data.austin_311.311_service_requests` LIMIT 10")
        .await;
    let err = format!("{:?}", result);
    assert!(result.is_err(), "Guard should have blocked the scan");
    assert!(err.contains("exceeds limit"), "Wrong error: {}", err);
}

#[tokio::test]
async fn test_run_query_small_query_succeeds() {
    let project = match std::env::var("BQ_PROJECT_ID") {
        Ok(p) => p,
        Err(_) => {
            println!("SKIP: BQ_PROJECT_ID not set");
            return;
        }
    };
    let config = BqConfig {
        project_id: project,
        credentials_path: String::new(), // ADC
        ..Default::default()
    };
    let mut adapter = BqAdapterLive::new();
    adapter.authenticate(&config).await.expect("authenticate() failed");

    let result = adapter
        .run_query("SELECT 1 AS one, 'x' AS letter")
        .await
        .expect("run_query() failed");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.schema.len(), 2);
    println!("row: {:?}, scanned {} bytes, job {}", result.rows[0], result.bytes_scanned, result.job_id);
}
