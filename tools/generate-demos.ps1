# Generate niche demo schema docs from free public BigQuery datasets.
# Prerequisites (one time):
#   gcloud auth application-default login
#   gcloud auth application-default set-quota-project <your-project-id>
#   $env:BQ_PROJECT_ID = "<your-project-id>"
# Cost: $0 - metadata API calls only, no rows read.

$ErrorActionPreference = "Stop"
$datasets = @(
    "thelook_ecommerce",    # e-commerce
    "cms_medicare",         # healthcare
    "crypto_bitcoin",       # fintech/crypto (nested RECORDs - shows flattening)
    "new_york_taxi_trips"   # logistics
)

cargo build --release --bin openclaw-swarm

foreach ($ds in $datasets) {
    Write-Host "=== $ds ===" -ForegroundColor Cyan
    & .\target\release\openclaw-swarm.exe bq-doc `
        --dataset "bigquery-public-data.$ds" `
        --out "docs/demo/$ds.md"
}

Write-Host "Done. Demos in docs/demo/" -ForegroundColor Green
