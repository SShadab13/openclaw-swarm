# Client Onboarding - BigQuery Access Setup

This takes about 10 minutes and requires no changes to your data or infrastructure.
You grant read-only access to metadata; you can revoke it at any time with one click.

## What access you are granting

| Role | What it allows | What it does NOT allow |
|------|----------------|------------------------|
| BigQuery Metadata Viewer | List datasets, tables, and column definitions | Reading any rows of your data |
| BigQuery Read Session User (optional, audits only) | Run read-only queries you approve in advance | Writing, modifying, or deleting anything |

For a schema documentation engagement, **Metadata Viewer alone is enough**.
Your data rows are never read.

## Setup steps (in your Google Cloud console)

1. Open IAM & Admin → Service Accounts → Create Service Account.
2. Name it `schema-audit` (or anything you like).
3. Grant it the role **BigQuery Metadata Viewer** on the project (or on specific datasets only, if you prefer).
4. Do NOT grant Editor, Owner, or any write role. We will refuse keys with write access.
5. Open the service account → Keys → Add Key → JSON → download.
6. Send the JSON key file through the agreed secure channel (not plain email).

## Costs

Metadata API calls are free in Google Cloud - a schema documentation run costs you $0 in BigQuery charges.
If a cost/usage audit is agreed later, queries run in **your** project against `INFORMATION_SCHEMA` (metadata views), and we pre-agree a byte-scan cap in writing before anything runs.

## Revoking access

IAM & Admin → Service Accounts → `schema-audit` → Disable (or Delete).
Access ends immediately.
We also delete our copy of the key at engagement end and confirm it in writing.

## What you receive

- A markdown/PDF document of every table: columns, types, modes, descriptions, partitioning, clustering, row counts.
- A walkthrough call to hand it over.
- The generated docs are yours; keep them in your own repo or wiki.
