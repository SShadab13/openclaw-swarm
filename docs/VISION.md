# VISION.md — Who This Is For, What It Isn't, Next 4 Weeks

## Who This Is For

**AI engineers who want a multi-agent system they actually own.**

- You want agent orchestration without Python framework bloat (no 500MB `node_modules`, no pip dependency hell)
- You want agent identity to matter (not just "role: coder" but "persona: coder × personality: tsundere × mood: frustrated")
- You want state that persists across sessions (SQLite, not in-memory dictionaries)
- You want a single binary you can deploy anywhere (8MB Rust executable, not a Docker stack)
- You want to extend your swarm with new personas, not rewrite it when your use case changes

**Not for:** People who want a chatbot wrapper around OpenAI API, or who need a managed SaaS with a web UI.

## What This Isn't (And Why)

### Not CrewAI
CrewAI is Python-based, role-string agents, sequential/graph execution, in-memory state. Good for quick prototypes. openclaw-swarm is for engineers who outgrow prototypes — Rust binary, persistent SQLite, combinatorial identity.

### Not AutoGen
AutoGen is Microsoft's Python multi-agent framework with heavy Azure coupling. Conversational agents, not phase-gated execution. Good for Microsoft shops. openclaw-swarm is for teams who want their engine to run on any machine without Azure lock-in.

### Not LangGraph
LangGraph is LangChain's graph-based orchestration — nodes and edges, state machines. Good for LLM-heavy workflows. openclaw-swarm is for engineers who need deterministic, explainable coordination (symbolic keyword matching, not vector search) and zero per-action API fees.

### Not a Managed SaaS
There is no cloud dashboard, no API key to buy, no per-agent pricing. You own the binary, you own the data, you own the execution. This is infrastructure, not a service.

## What We Have That They Don't

1. **M×N Personality Matrix** — 9 personas × 9 personalities = 81 combinatorial agents. Not "roles," *identities.*
2. **Error Journal** — Agents learn from failures. Pattern matching, not just error logging.
3. **Letters & Diary** — Agents communicate and reflect. Not just logs — *relationships.*
4. **Phase-Gated Execution** — Epic → Story → Phase → Task with recovery primitives (InsertPrereq, Substitute, Rewire, Bypass, Escalate, Rebind).
5. **Caveman Compression** — ~30% token reduction in agent communications.
6. **Single Binary** — `cargo build --release` produces one 8MB file. Zero runtime dependencies.
7. **Data Engineering Extension** — BigQuery adapter with cost guardrails, zero-cost on public datasets.

## Next 4 Weeks

### Week 1: Foundation
- **Sunday ✅ DONE:** 4 DE personas + BQ adapter stub pushed to GitHub
- **Monday:** GCP project + BQ auth (service account, `gcp-bigquery-client` crate)
- **Tuesday:** `list_datasets()` + `get_schema()` tested against `bigquery-public-data.github_repos`
- **Wednesday:** `run_query()` with cost guardrails (dry-run preflight)
- **Thursday:** Wire `schema_discoverer` persona end-to-end
- **Friday:** Journal write-up, no new code

### Week 2: Demo
- **Monday–Friday:** Polish, edge cases, mock tests
- **Saturday:** Full `schema_discoverer` scan of `github_repos` dataset → capture output → first demoable artifact

### Week 3: Content
- First LinkedIn post: "Two years ago a colleague and I started building openclaw-swarm... This weekend I extended it for BigQuery automation. Here's what 4 specialized agents look like running schema discovery on github_repos."
- Start positioning: "AI Agent Engineer — builder of openclaw-swarm, applying multi-agent systems to data engineering."

### Week 4: Client-Ready
- Package the DE extension as a standalone module
- Write client-facing documentation (not just repo README)
- Identify first freelance target: data engineering teams needing schema automation

## Long-Term (3–6 Months)

- Add more data source adapters (Snowflake, Databricks, Postgres)
- Build a portfolio of 3–5 public dataset demos (GitHub repos, Hacker News, Stack Overflow, public crypto data)
- Position as "AI Agent Engineer" — not just "Rust developer" or "data engineer"
- Potential revenue: freelance contracts for custom agent orchestration, paid workshops, content

---

*Written 2026-05-23. This file exists because future-you will forget the why. Don't let them.*
