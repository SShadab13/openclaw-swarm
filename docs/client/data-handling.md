# Data Handling Policy

How client data is treated in every engagement.
This policy is part of the engagement agreement.

## Tier 1 - Metadata only (default for all engagements)

- We access schema metadata only: dataset names, table names, column names/types, partitioning, row counts.
- No table rows are ever read. The access role you grant (BigQuery Metadata Viewer) makes reading rows technically impossible, not just promised.
- No LLM or AI service receives your metadata unless separately agreed in writing.
- BigQuery cost to you: $0 (metadata APIs are free).

## Tier 2 - AI processing inside YOUR cloud

Applies when the engagement includes AI-generated documentation, lineage inference, or data quality analysis.

- All AI calls run inside your own cloud tenancy: Vertex AI in your GCP project, or Amazon Bedrock in your AWS account.
- Your data and metadata never leave your environment; we work through IAM access you grant and can revoke.
- You see the AI usage costs directly on your own cloud bill - no markup, no hidden processing.
- We never route your data through our own AI accounts or third-party AI APIs.

## Tier 3 - Processing on our systems (exception, never default)

Only when Tiers 1-2 are technically impossible, and only with a signed data processing addendum. Requires:

- A written scope listing the exact tables/extracts involved.
- PII columns excluded or masked before transfer.
- Data stored encrypted, on one machine, never in cloud sync folders.
- Deletion within 14 days of engagement end, confirmed in writing.

## In every tier

- Access credentials are stored outside cloud-synced folders, never committed to git, and deleted at engagement end.
- Reports contain schema information only - never data values - unless you request samples in writing.
- Sub-processors: none. One person does the work; nothing is outsourced.
- Breach notification: any suspected credential compromise is reported to you within 24 hours.
