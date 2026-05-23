//! BigQuery Adapter — Data Engineering Persona Interface
//!
//! Stub for week 1. Implements:
//! - Service account auth
//! - Dataset/schema discovery
//! - Query execution with cost guardrails
//! - Audit log access for lineage extraction
//!
//! TODO (Mon): Add `google-cloud-bigquery` crate to Cargo.toml
//! TODO (Mon): Implement auth from service-account JSON key
//! TODO (Tue): Implement list_datasets() + get_schema()
//! TODO (Wed): Implement run_query() with max_bytes_scanned guard
//! TODO (Thu): Wire schema_discoverer persona end-to-end

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// BigQuery connection configuration
#[derive(Debug, Clone, Deserialize)]
pub struct BqConfig {
    /// GCP project ID
    pub project_id: String,
    /// Path to service-account JSON key file
    pub credentials_path: String,
    /// Default dataset (optional)
    pub default_dataset: Option<String>,
    /// Maximum bytes a single query may scan (cost guardrail)
    pub max_bytes_scanned: Option<i64>,
    /// Location/region (e.g. "US", "EU", "asia-south1")
    pub location: Option<String>,
}

impl Default for BqConfig {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            credentials_path: String::new(),
            default_dataset: None,
            max_bytes_scanned: Some(100_000_000_000), // 100 GB default guard
            location: Some("US".to_string()),
        }
    }
}

/// Column metadata for a BigQuery table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BqColumn {
    pub name: String,
    pub data_type: String,
    pub mode: String, // REQUIRED, NULLABLE, REPEATED
    pub description: Option<String>,
    pub is_partitioned: bool,
    pub is_clustered: bool,
}

/// Table schema snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BqTableSchema {
    pub dataset_id: String,
    pub table_id: String,
    pub columns: Vec<BqColumn>,
    pub partition_column: Option<String>,
    pub clustering_columns: Vec<String>,
    pub num_bytes: i64,
    pub num_rows: i64,
    pub last_modified: DateTime<Utc>,
    pub description: Option<String>,
}

/// Dataset metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BqDataset {
    pub dataset_id: String,
    pub location: String,
    pub description: Option<String>,
    pub labels: Vec<(String, String)>,
    pub access_entries: Vec<String>,
}

/// Query result wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BqQueryResult {
    pub job_id: String,
    pub query: String,
    pub bytes_scanned: i64,
    pub rows: Vec<serde_json::Value>,
    pub schema: Vec<BqColumn>,
    pub execution_time_ms: i64,
}

/// Audit log entry for lineage extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BqAuditEntry {
    pub timestamp: DateTime<Utc>,
    pub job_id: String,
    pub query_text: String,
    pub user_email: String,
    pub referenced_tables: Vec<String>,
    pub destination_table: Option<String>,
    pub bytes_processed: i64,
}

/// Cost guardrail violation
#[derive(Debug, thiserror::Error)]
pub enum BqGuardError {
    #[error("Query would scan {0} bytes, exceeds limit {1}")]
    ScanLimitExceeded(i64, i64),
    #[error("No query text provided")]
    EmptyQuery,
    #[error("Invalid table reference: {0}")]
    InvalidTable(String),
}

/// BigQuery adapter trait — all data engineering personas interact through this interface
#[async_trait]
pub trait BigQueryAdapter: Send + Sync {
    /// Authenticate with GCP using service-account JSON
    async fn authenticate(&mut self, config: &BqConfig) -> Result<()>;

    /// List all datasets in the configured project
    async fn list_datasets(&self) -> Result<Vec<BqDataset>>;

    /// Get full schema for a specific table
    /// Format: "dataset.table" or "project.dataset.table"
    async fn get_schema(&self, table_ref: &str) -> Result<BqTableSchema>;

    /// Execute a query with automatic cost protection
    /// Returns BqGuardError if query exceeds max_bytes_scanned
    async fn run_query(&self, sql: &str) -> Result<BqQueryResult>;

    /// Fetch audit logs / job history for lineage extraction
    async fn get_audit_logs(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<BqAuditEntry>>;
}

/// Concrete adapter implementation (placeholder — week 1 stub)
pub struct BqAdapterLive {
    // TODO: google-cloud-bigquery client
    // TODO: authenticated project handle
    config: Option<BqConfig>,
}

impl BqAdapterLive {
    pub fn new() -> Self {
        Self { config: None }
    }
}

#[async_trait]
impl BigQueryAdapter for BqAdapterLive {
    async fn authenticate(&mut self, config: &BqConfig) -> Result<()> {
        // TODO(Mon): Load service-account JSON from credentials_path
        // TODO(Mon): Initialize google-cloud-bigquery client
        // TODO(Mon): Validate project_id access
        self.config = Some(config.clone());
        tracing::info!("BQ auth stub: would authenticate to project {}", config.project_id);
        Ok(())
    }

    async fn list_datasets(&self) -> Result<Vec<BqDataset>> {
        // TODO(Tue): Call BQ REST API: GET /projects/{project}/datasets
        // TODO(Tue): Parse response into Vec<BqDataset>
        tracing::info!("BQ list_datasets stub");
        Ok(vec![])
    }

    async fn get_schema(&self, table_ref: &str) -> Result<BqTableSchema> {
        // TODO(Tue): Parse table_ref into project/dataset/table
        // TODO(Tue): Call BQ API: GET /tables/{ref}
        // TODO(Tue): Fetch columns, partitioning, clustering
        tracing::info!("BQ get_schema stub for {}", table_ref);
        Ok(BqTableSchema {
            dataset_id: "stub".to_string(),
            table_id: table_ref.to_string(),
            columns: vec![],
            partition_column: None,
            clustering_columns: vec![],
            num_bytes: 0,
            num_rows: 0,
            last_modified: Utc::now(),
            description: None,
        })
    }

    async fn run_query(&self, sql: &str) -> Result<BqQueryResult> {
        // TODO(Wed): Pre-flight dry-run to estimate bytes_scanned
        // TODO(Wed): Check against config.max_bytes_scanned
        // TODO(Wed): Execute actual query via BQ client
        // TODO(Wed): Stream results into Vec<serde_json::Value>
        tracing::info!("BQ run_query stub: {}", sql);
        Ok(BqQueryResult {
            job_id: "stub".to_string(),
            query: sql.to_string(),
            bytes_scanned: 0,
            rows: vec![],
            schema: vec![],
            execution_time_ms: 0,
        })
    }

    async fn get_audit_logs(
        &self,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<BqAuditEntry>> {
        // TODO(Wed): Query INFORMATION_SCHEMA.JOBS_BY_* or Stackdriver logs
        // TODO(Wed): Parse query text for table references (regex or AST walk)
        tracing::info!("BQ get_audit_logs stub");
        Ok(vec![])
    }
}

/// Mock adapter for unit testing personas without real GCP credentials
pub struct BqAdapterMock {
    pub datasets: Vec<BqDataset>,
    pub schemas: Vec<BqTableSchema>,
    pub query_results: Vec<BqQueryResult>,
    pub audit_logs: Vec<BqAuditEntry>,
}

#[async_trait]
impl BigQueryAdapter for BqAdapterMock {
    async fn authenticate(&mut self, _config: &BqConfig) -> Result<()> {
        Ok(())
    }

    async fn list_datasets(&self) -> Result<Vec<BqDataset>> {
        Ok(self.datasets.clone())
    }

    async fn get_schema(&self, table_ref: &str) -> Result<BqTableSchema> {
        self.schemas
            .iter()
            .find(|s| s.table_id == table_ref)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Mock: table {} not found", table_ref))
    }

    async fn run_query(&self, sql: &str) -> Result<BqQueryResult> {
        self.query_results
            .iter()
            .find(|r| r.query == sql)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Mock: query not pre-staged: {}", sql))
    }

    async fn get_audit_logs(
        &self,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<BqAuditEntry>> {
        Ok(self.audit_logs.clone())
    }
}
