# OpenClaw Swarm

**Agent swarm engine with personality-driven M×N matrix architecture.**

Built in Rust. Forked from the philosophy of [Pulse](https://www.instagram.com/reel/DYBSB8NsFYE/) — build the engine first, the IDE last.

## Philosophy

- **Queen** (Ayan/OpenClaw) assigns personalities (souls) to personas (agents)
- **MxN matrix**: M personalities × N personas = infinite combinations
- **Sandbox**: Agents build behind closed doors (Git branch + VM), ship when ready
- **Letters**: Agents communicate with each other
- **Diary**: Private agent journals
- **Error Journal**: Structured learning from failures
- **Caveman**: Token compression for efficiency

## Architecture

```
Queen (Sovereign)
  └── Coordinator (Chamberlain)
      ├── Agent A (Persona: Coder + Personality: Tsundere + Mood: Frustrated)
      │   ├── Journal: "Tried to refactor XP engine. Broke streaks."
      │   ├── Letter to Queen: "Backend is incompetent."
      │   └── Error Log: "Type mismatch at services/xp.ts:47"
      │
      ├── Agent B (Persona: Tester + Personality: Sadist + Mood: Excited)
      │   └── Writing: "Found 7 bugs! Let me show you how..."
      │
      └── Agent C (Persona: Architect + Personality: Meticulous + Mood: Calm)
          └── Reading: "How to Read a Book" (Adler)
```

## Personas (13)

### Core (9)

| ID | Role | Base Personality |
|----|------|----------------|
| `architect` | System design | Meticulous |
| `coder` | Implementation | Fast/loose |
| `tester` | Bug hunting | Sadist (cheerful) |
| `frontend` | UI/UX | Visual |
| `backend` | API/services | Pedantic |
| `designer` | Product design | Perfectionist |
| `devops` | Infrastructure | Paranoid |
| `mlops` | ML systems | Data-obsessed |
| `agentic_ops` | Agent orchestration | Meta |

### Data Engineering (4)

| ID | Role | Base Personality |
|----|------|----------------|
| `schema_discoverer` | Scan datasets, infer types, detect drift | Meticulous |
| `lineage_extractor` | Trace data flows, map dependencies | Paranoid |
| `doc_writer` | Generate data contracts, column docs | Meticulous |
| `data_validator` | Assert row counts, nulls, ranges, compliance | Sadist (cheerful) |

## Personalities (9 souls)

| ID | Description | Token Cost |
|----|-------------|-----------|
| `tsundere` | Harsh but cares | Medium |
| `honest` | No filter, brutal | Low |
| `meticulous` | Edge cases, types | High |
| `sadist_cheerful` | Loves breaking things | Low |
| `confused` | Asks clarifying questions | Medium |
| `angry` | Frustrated by debt | Low |
| `cheerful` | Optimistic, encouraging | Medium |
| `moody` | Unpredictable, brilliant | Variable |
| `paranoid` | Sees threats everywhere | Medium |

## Databases

- `swarm_tasks.db` — Task logs, assignments, letters, diary
- `error_journal.db` — Error patterns, root causes, solutions
- `knowledge.db` — Book ingestion, mental models (future)

## CLI Usage

```bash
# Initialize
cargo run -- init

# Create a task with auto-assigned swarm
cargo run -- task --name "Day Seal Feature" --description "Implement day seal" --task-type sdlc_feature

# Start the swarm
cargo run -- start --task-id <uuid>

# Check status
cargo run -- status --task-id <uuid>

# Ship when ready
cargo run -- ship --task-id <uuid>
```

## Phased Development Orchestration (v1.0)

The swarm now supports hierarchical, gated-phase development:

```
Epic → Stories → Phases → Tasks
```

**5 Phases per Story:**
1. **Planning** — Requirements analysis, scoping, sizing (PlanningAgent)
2. **Design** — Architecture, schema, API contracts (System Agent)
3. **Implementation** — Parallel coding by specialty agents (Coder A/B/C)
4. **Review** — 3-dimension parallel review (Security, Performance, Conventions)
5. **Ship** — Merge, commit, graphify (Queen + Sandbox)

**Phase Gates:**
- Each phase requires approval before proceeding
- Approve → next phase
- Request Changes → surgical fix → back to Implementation
- Reject → replan with 6 recovery primitives (InsertPrereq, Substitute, Rewire, Bypass, Escalate, Rebind)

**Agent Selection — Queen Knows Her Court:**
- 12+ personas with capability YAML declarations
- Symbolic keyword matching (deterministic, explainable, fast)
- Zero vector search, zero API dependencies

**Context Protocol — 5-Section YAML Header:**
- Goal, Circle of Competence, Role, Expectations, Relevant History
- Tiered phase briefing — activated episodes, not full history dump

### Dashboard (v2.0)

```bash
# Start web dashboard
cargo run -- serve --port 8080
# Open http://localhost:8080
```

**Screens:**
- **Overview** — Hero + metrics + attention list + active epics
- **Epic Board** — Expandable epic cards, story grid, search/filter
- **Story Phase** — Phase timeline, activity stream, agent cards, action bar
- **Phase Detail** — Artifacts, letters, commits, review report (tabbed)
- **Agent Health** — Health scores, error counts, status dots

**Features:**
- Real-time SSE updates with auto-reconnect
- Mock mode for visual testing
- Keyboard shortcuts (Cmd+K search, a/r/p actions, ? help, Esc close)
- Toast notifications
- Gold-primary color palette (stolen from Kimi's React build)

### CLI — New Commands

```bash
# Web dashboard
openclaw-swarm serve --port 8080

# Phase management (via API)
curl http://localhost:8080/api/phases/{story_id}
curl -X POST http://localhost:8080/api/phase/{phase_id}/approve
curl -X POST http://localhost:8080/api/phase/{phase_id}/reject
curl -X POST http://localhost:8080/api/phase/{phase_id}/replan
```

## Build

```bash
cargo build --release
```

**Binary:** ~8 MB (single file, zero runtime dependencies)

## Test

```bash
# Integration tests — 12 tests covering full phase lifecycle
cargo test --test integration

# Unit tests — 14 tests
cargo test --lib
```

## Data Engineering Extension

BigQuery adapter for data engineering personas. Zero-cost on BigQuery free tier (1 TB queries/month + 10 GB storage).

### Adapter Interface (`src/adapters/bq_adapter.rs`)

```rust
pub trait BigQueryAdapter {
    async fn authenticate(&mut self, config: &BqConfig) -> Result<()>;
    async fn list_datasets(&self) -> Result<Vec<BqDataset>>;
    async fn get_schema(&self, table_ref: &str) -> Result<BqTableSchema>;
    async fn run_query(&self, sql: &str) -> Result<BqQueryResult>;
    async fn get_audit_logs(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<BqAuditEntry>>;
}
```

**Cost guardrails:** Every query runs a dry-run first. If `bytes_scanned > max_bytes_scanned` (default 100 GB), the query is rejected before execution.

**Week 1–2 Roadmap:**
- Sun: Persona YAMLs + adapter stub (DONE)
- Mon: BQ auth (service account JSON)
- Tue: `list_datasets()` + `get_schema()` against `bigquery-public-data.github_repos`
- Wed: `run_query()` with cost protection
- Thu: Wire `schema_discoverer` end-to-end
- Fri: Journal write-up
- Sat (wk2): Full dataset scan demo

## Install for Any CLI Agent

```bash
# Clone
git clone https://github.com/SShadab13/openclaw-swarm.git
cd openclaw-swarm

# Build release binary (~8 MB, single file)
cargo build --release

# Binary lands at:
# ./target/release/openclaw-swarm

# Or install via cargo
cargo install --path .

# Now available as `openclaw-swarm` in PATH
```

**For Claude Code / Kimi / Cursor / Gemini CLI:**

These agents can read the compiled binary, the persona YAMLs, and the adapter traits to understand swarm capabilities. Point them at `personas/` and `src/adapters/` — the YAML + Rust interface is self-documenting.

## License

MIT
