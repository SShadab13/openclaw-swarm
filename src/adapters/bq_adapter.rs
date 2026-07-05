//! BigQuery Adapter — Data Engineering Persona Interface
//!
//! Implemented: service-account auth, list_datasets(), list_tables(), get_schema().
//!
//! TODO: Implement run_query() with max_bytes_scanned guard
//! TODO: Implement get_audit_logs() for lineage extraction
//! TODO: Wire schema_discoverer persona end-to-end

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use gcp_bigquery_client;
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

/// Serialize a BQ FieldType to its API string ("STRING", "RECORD", ...).
fn field_type_str(t: &gcp_bigquery_client::model::field_type::FieldType) -> String {
    serde_json::to_string(t)
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_else(|_| format!("{:?}", t).to_uppercase())
}

/// Flatten a (possibly nested) field list into columns with dotted names.
/// RECORD parents are kept as rows so the doc shows the structure, and
/// their children follow as "parent.child".
fn flatten_fields(
    fields: Vec<gcp_bigquery_client::model::table_field_schema::TableFieldSchema>,
    prefix: &str,
    partition_column: &Option<String>,
    clustering_columns: &[String],
) -> Vec<BqColumn> {
    let mut out = Vec::new();
    for f in fields {
        let name = if prefix.is_empty() {
            f.name.clone()
        } else {
            format!("{}.{}", prefix, f.name)
        };
        out.push(BqColumn {
            is_partitioned: partition_column.as_deref() == Some(name.as_str()),
            is_clustered: clustering_columns.contains(&name),
            name: name.clone(),
            data_type: field_type_str(&f.r#type),
            mode: f.mode.clone().unwrap_or_else(|| "NULLABLE".to_string()),
            description: f.description.clone(),
        });
        if let Some(children) = f.fields {
            out.extend(flatten_fields(children, &name, partition_column, clustering_columns));
        }
    }
    out
}

/// Parse a table reference into (project, dataset, table).
/// Accepts "dataset.table" (project from default) or "project.dataset.table".
pub fn parse_table_ref(table_ref: &str, default_project: &str) -> Result<(String, String, String)> {
    let parts: Vec<&str> = table_ref.split('.').collect();
    match parts.as_slice() {
        [dataset, table] if !dataset.is_empty() && !table.is_empty() => Ok((
            default_project.to_string(),
            dataset.to_string(),
            table.to_string(),
        )),
        [project, dataset, table] if !project.is_empty() && !dataset.is_empty() && !table.is_empty() => {
            Ok((project.to_string(), dataset.to_string(), table.to_string()))
        }
        _ => Err(BqGuardError::InvalidTable(table_ref.to_string()).into()),
    }
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

    /// List table IDs in a dataset (configured project)
    async fn list_tables(&self, dataset_id: &str) -> Result<Vec<String>>;

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

/// Concrete adapter implementation backed by the real GCP BigQuery client.
pub struct BqAdapterLive {
    client: Option<gcp_bigquery_client::Client>,
    config: Option<BqConfig>,
}

impl BqAdapterLive {
    pub fn new() -> Self {
        Self {
            client: None,
            config: None,
        }
    }
}

#[async_trait]
impl BigQueryAdapter for BqAdapterLive {
    async fn authenticate(&mut self, config: &BqConfig) -> Result<()> {
        // Empty credentials_path = use Application Default Credentials
        // (`gcloud auth application-default login`)
        let client = if config.credentials_path.is_empty() {
            gcp_bigquery_client::Client::from_application_default_credentials().await?
        } else {
            gcp_bigquery_client::Client::from_service_account_key_file(&config.credentials_path)
                .await?
        };
        self.client = Some(client);
        self.config = Some(config.clone());
        tracing::info!("BQ auth OK: project={}", config.project_id);
        Ok(())
    }

    async fn list_datasets(&self) -> Result<Vec<BqDataset>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated — call authenticate() first"))?;
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No BqConfig set"))?;

        let result = client
            .dataset()
            .list(&config.project_id, Default::default())
            .await?;

        let datasets = result
            .datasets
            .into_iter()
            .map(|d| BqDataset {
                dataset_id: d.dataset_reference.dataset_id,
                location: d.location.unwrap_or_default(),
                description: d.friendly_name,
                labels: d
                    .labels
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
                access_entries: vec![],
            })
            .collect();

        Ok(datasets)
    }

    async fn list_tables(&self, dataset_id: &str) -> Result<Vec<String>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated — call authenticate() first"))?;
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No BqConfig set"))?;

        let mut tables = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut opts =
                gcp_bigquery_client::table::ListOptions::default().max_results(1000);
            if let Some(t) = &page_token {
                opts = opts.page_token(t.clone());
            }
            let result = client
                .table()
                .list(&config.project_id, dataset_id, opts)
                .await?;
            tables.extend(
                result
                    .tables
                    .unwrap_or_default()
                    .into_iter()
                    .map(|t| t.table_reference.table_id),
            );
            page_token = result.next_page_token;
            if page_token.is_none() {
                break;
            }
        }
        Ok(tables)
    }

    async fn get_schema(&self, table_ref: &str) -> Result<BqTableSchema> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated — call authenticate() first"))?;
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No BqConfig set"))?;

        let (project, dataset, table) = parse_table_ref(table_ref, &config.project_id)?;

        let t = client
            .table()
            .get(&project, &dataset, &table, None)
            .await?;

        let partition_column = t
            .time_partitioning
            .as_ref()
            .and_then(|tp| tp.field.clone());
        let clustering_columns: Vec<String> = t
            .clustering
            .as_ref()
            .and_then(|c| c.fields.clone())
            .unwrap_or_default();

        let columns = flatten_fields(
            t.schema.fields.unwrap_or_default(),
            "",
            &partition_column,
            &clustering_columns,
        );

        let last_modified = t
            .last_modified_time
            .as_deref()
            .and_then(|ms| ms.parse::<i64>().ok())
            .and_then(|ms| chrono::DateTime::<Utc>::from_timestamp_millis(ms))
            .unwrap_or_else(Utc::now);

        Ok(BqTableSchema {
            dataset_id: dataset,
            table_id: table,
            columns,
            partition_column,
            clustering_columns,
            num_bytes: t.num_bytes.as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            num_rows: t.num_rows.as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            last_modified,
            description: t.description,
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

    async fn list_tables(&self, dataset_id: &str) -> Result<Vec<String>> {
        Ok(self
            .schemas
            .iter()
            .filter(|s| s.dataset_id == dataset_id)
            .map(|s| s.table_id.clone())
            .collect())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(ds: &str, t: &str) -> BqTableSchema {
        BqTableSchema {
            dataset_id: ds.to_string(),
            table_id: t.to_string(),
            columns: vec![],
            partition_column: None,
            clustering_columns: vec![],
            num_bytes: 0,
            num_rows: 0,
            last_modified: Utc::now(),
            description: None,
        }
    }

    #[test]
    fn test_parse_table_ref_two_parts_uses_default_project() {
        let (p, d, t) = parse_table_ref("austin_311.service_requests", "default-proj").unwrap();
        assert_eq!(p, "default-proj");
        assert_eq!(d, "austin_311");
        assert_eq!(t, "service_requests");
    }

    #[test]
    fn test_parse_table_ref_three_parts() {
        let (p, d, t) =
            parse_table_ref("bigquery-public-data.austin_311.311_service_requests", "x").unwrap();
        assert_eq!(p, "bigquery-public-data");
        assert_eq!(d, "austin_311");
        assert_eq!(t, "311_service_requests");
    }

    #[test]
    fn test_parse_table_ref_invalid() {
        assert!(parse_table_ref("justatable", "x").is_err());
        assert!(parse_table_ref("a.b.c.d", "x").is_err());
        assert!(parse_table_ref("", "x").is_err());
    }

    #[test]
    fn test_flatten_fields_nested_record() {
        use gcp_bigquery_client::model::field_type::FieldType;
        use gcp_bigquery_client::model::table_field_schema::TableFieldSchema;

        let mut device = TableFieldSchema::new("device", FieldType::Record);
        device.fields = Some(vec![TableFieldSchema::new("browser", FieldType::String)]);
        let mut totals = TableFieldSchema::new("totals", FieldType::Record);
        totals.mode = Some("REPEATED".to_string());
        totals.fields = Some(vec![TableFieldSchema::new("visits", FieldType::Integer), device]);

        let cols = flatten_fields(
            vec![TableFieldSchema::new("id", FieldType::String), totals],
            "",
            &None,
            &[],
        );

        let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["id", "totals", "totals.visits", "totals.device", "totals.device.browser"]
        );
        let t = cols.iter().find(|c| c.name == "totals").unwrap();
        assert_eq!(t.data_type, "RECORD");
        assert_eq!(t.mode, "REPEATED");
        let leaf = cols.iter().find(|c| c.name == "totals.device.browser").unwrap();
        assert_eq!(leaf.data_type, "STRING");
    }

    #[tokio::test]
    async fn test_mock_list_tables_filters_by_dataset() {
        let mock = BqAdapterMock {
            datasets: vec![],
            schemas: vec![schema("ds1", "t1"), schema("ds2", "t2"), schema("ds1", "t3")],
            query_results: vec![],
            audit_logs: vec![],
        };
        let tables = mock.list_tables("ds1").await.unwrap();
        assert_eq!(tables, vec!["t1".to_string(), "t3".to_string()]);
    }
}
