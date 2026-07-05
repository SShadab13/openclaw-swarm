# openclaw-swarm - Agent Memory

Single memory file for all agents (Kimi, Claude, Codex, Cursor). CLAUDE.md is a symlink to this file - edit either, same file.
Kun Chen discipline: this file grows from corrections. When the owner corrects you, add the learning here so no agent repeats the mistake.

## What This Is

Rust CLI multi-agent swarm engine. Personality-driven MxN matrix (M personalities x N personas), phase-gated execution, Error Journal, TUI + web dashboard, SQLite state. ~2200 lines, 23 source files.

- **GitHub:** github.com/SShadab13/openclaw-swarm
- **Owner:** Shadab (s.shadab.dav9@gmail.com)
- **Why it exists:** side hustle - productised AI data engineering tool for SMBs. Deep knowledge, decisions, and roadmap live in the Second Brain vault (`I:\My Drive\Second Brain`, WSL: `/mnt/i/My Drive/Second Brain`) - check its `index.md` and `Master Context.md` for context beyond code.

## Tech Stack (enforced)

- Rust 2021, Cargo. No deprecated methods or alternative libraries without owner approval.
- SQLite for state (`db/`), YAML personas (`personas/`), personalities (`personalities/`).

## Build & Test

```
cargo build
cargo test
cargo run -- --help
```

Run any of these locally after changes - verify it compiles before saying done.

## Layout

```
src/
├── main.rs, lib.rs, models.rs
├── queen.rs, coordinator.rs      ← orchestration (Queen assigns personalities to personas)
├── execution_loop.rs, task_fsm.rs, phases/
├── error_journal.rs, sandbox.rs, swarm_bus.rs
├── adapters/                     ← external integrations (bq_adapter.rs = BigQuery)
├── personas/, personalities/     ← YAML definitions (13 personas)
└── dashboard.rs, web_dashboard.rs
docs/superpowers/plans/           ← implementation plans
```

## Current Focus (updated 2026-07-05 late night)

BQ toolkit is LIVE and demoed. Working end-to-end with gcloud ADC (project `openclaw-bq-shadab13`, sandbox, $0):
- `bq-doc` - markdown schema docs (nested RECORDs flattened, paginated table listing, TOC summary)
- `bq-snapshot` + `bq-diff` - schema-change watcher (JSON snapshots -> markdown changelog)
- 4 niche demo docs + PDF sample in `docs/demo/`; README leads with the toolkit; client docs in `docs/client/`
- 52/52 tests green. `cargo run` needs `--bin openclaw-swarm`.
Auth note: crate's `from_application_default_credentials()` only works on GCE VMs; adapter resolves gcloud's ADC file itself (see `adc_well_known_path`). `from_authorized_user_secret()` takes a file PATH despite the name.
- `bq-lint` - metadata-only dataset auditor (cost traps + doc gaps); live demo report `docs/demo/cms_medicare_lint.md` (5 real warnings)
- `run_query()` real with double cost guard (dry-run estimate + server-side maximum_bytes_billed). Live query tests gate on `BQ_PROJECT_ID` env (use ADC).
Only stub left: `get_audit_logs()` (lineage extraction - needs client-project JOBS access, can't demo on public data).
Business docs: pricing/tiers/legal in vault `wiki/syntheses/BQ DE Service - Client Playbook.md`.

## Known Defects (eval 2026-07-05)

1. ~~`test_size_analysis` FAILS on main~~ FIXED (2f54287).
2. ~~`ClaudeRunner` hardcodes `bypassPermissions`~~ FIXED (dabc552): default `acceptEdits`, bypass needs explicit `with_bypass_permissions()`.
3. ~~`&output[..500]` UTF-8 panic in execution_loop.rs~~ FIXED (dabc552): `truncate_chars()`.
4. ~~`ClaudeRunner.timeout` never enforced~~ FIXED (dabc552): `run_with_timeout()` kills at deadline.
5. SwarmBus stream subscribed then unused in `execute_agent_step` (`_stream`) - real comm channel is DB letters; pub/sub currently decorative. OPEN - owner decision.
6. ~~CLI output regex scraping~~ FIXED (dabc552): `--output-format json` with raw-stdout fallback.
7. `ClaudeRunner.execute()` is sync and blocks the tokio worker for up to its timeout when called from async adapters. OPEN - pre-existing, low priority.

## Rules

- Never chain commit/push after piped test output: `cargo test | tail && git commit` masks failures (pipe exit code = tail's, not cargo's). Check `${PIPESTATUS[0]}` or run cargo bare first. A broken commit reached origin/main this way on 2026-07-05 (a7d78f7, fixed 1f6d04c).
- Bug fixes: reproduce end-to-end first (run the actual CLI flow), then fix. Unit test alone insufficient.
- Technical decisions: ignore development cost; prefer quality, robustness, maintainability.
- Never modify: `.env` files, `db/` contents (agent state), `target/`.
- Before new utility, scan `src/` for existing similar function.
- Knowledge graph exists at `graphify-out/` - for architecture questions read `graphify-out/GRAPH_REPORT.md` or `graphify query "<question>"` before grepping raw files. After code changes: `graphify update .`
