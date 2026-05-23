# PRD: Swarm Execution Bridge

**Project:** openclaw-swarm v0.1.0 → v0.2.0  
**Feature:** Persistent Execution Workers + Sequential Task Decomposition  
**Date:** May 9, 2026  
**Owner:** Shadab (s.shadab.dav9@gmail.com)  
**Builder:** Ayan (OpenClaw Coordinator)

---

## 1. Problem Statement

### Current State (v0.1.0)
The swarm assigns personas to tasks but **cannot execute their work automatically**. Subagents spawned via `sessions_spawn` timeout at ~2.5m regardless of `timeoutSeconds` parameter. A multi-file feature (e.g., TUI dashboard with `dashboard.rs` + `lib.rs` + `main.rs` changes) cannot complete within this window.

**Impact:**
- Swarm can only write single files before dying
- Human must manually wire modules, fix imports, rebuild
- True "fire and forget" swarm automation is blocked
- Multi-file features (>3 files) require human intervention

### Target State (v0.2.0)
Swarm decomposes large tasks into sub-tasks that fit within timeout windows, executes them sequentially with state persistence, and auto-wires results without human intervention.

---

## 2. Goals

| # | Goal | Metric |
|---|------|--------|
| G1 | Reduce human intervention for multi-file features from 100% to <20% | % of features requiring human wiring |
| G2 | Enable 10+ file features to complete autonomously | Max files in single swarm task |
| G3 | Maintain 100% compilation success rate | `cargo check` passes after every task |
| G4 | Sub-agent recovery: retry failed chunks without losing context | Retry success rate |

---

## 3. Non-Goals

- NOT replacing `sessions_spawn` with a different spawn mechanism
- NOT building a custom agent runtime (still use OpenClaw subagents)
- NOT removing the 2.5m timeout (it's a hard system limit)
- NOT implementing full distributed computing (single host only)

---

## 4. User Stories

1. **As swarm owner,** I want to run `openclaw-swarm start --task-id X` and have the swarm complete a 5-file feature without me touching code, so I can focus on higher-level decisions.

2. **As swarm coordinator,** I want timed-out subagents to leave their partial work in a known location, so the next subagent can continue from where they left off.

3. **As code agent,** I want to see what files my predecessor already created, so I don't recreate them or break their imports.

---

## 5. Architecture

### 5.1 Core Concept: Task Decomposition

```
Large Task ("Build auth system" - 8 files)
    ├── Phase 1: Scaffold (2 files) → Agent A → 2m ✅
    ├── Phase 2: Core Logic (3 files) → Agent B → 2.5m ✅  
    ├── Phase 3: Integration (2 files) → Agent C → 2m ✅
    └── Phase 4: Tests + Wiring (1 file) → Agent D → 1.5m ✅
```

Each phase produces:
- **Files written** (tracked in DB)
- **Build status** (compiled or not)
- **Errors encountered** (for error journal)
- **Letter to next agent** (handoff context)

### 5.2 State Machine

```
[CREATED] → [DECOMPOSED] → [PHASE_1_RUNNING] → [PHASE_1_DONE]
    ↓              ↓              ↓ (timeout)           ↓
  Queen      Coordinator     Retry with context     [PHASE_2_RUNNING]
  assigns     breaks into                            ↓
  swarm       sub-tasks                        [ALL_PHASES_DONE]
                                                      ↓
                                                 [AUTO_WIRE]
                                                      ↓
                                                 [COMPILED]
                                                      ↓
                                                 [SHIPPED]
```

### 5.3 Components

| Component | Responsibility | File |
|-----------|---------------|------|
| **TaskDecomposer** | Breaks tasks into phases based on file count + complexity | `src/execution/task_decomposer.rs` |
| **PhaseExecutor** | Runs a single phase via `sessions_spawn`, tracks timeout | `src/execution/phase_executor.rs` |
| **StateManager** | Persists phase results to DB, provides context to next phase | `src/execution/state_manager.rs` |
| **AutoWirer** | Fixes imports, adds `mod` declarations, runs `cargo check` | `src/execution/auto_wirer.rs` |
| **ErrorRecoverer** | Retries failed phases, reads error journal for patterns | `src/execution/error_recoverer.rs` |
| **ProgressTracker** | Dashboard integration — shows phase-level progress | `src/execution/progress_tracker.rs` |

---

## 6. Epic Breakdown

### Epic 1: Task Decomposition Engine
**Goal:** Break multi-file tasks into sequential phases that fit within timeout.

**Stories:**
- E1S1: Parse task description to estimate file count
- E1S2: Create phase schema (phase_id, files[], agent_type, estimated_time)
- E1S3: Store decomposition plan in DB before execution
- E1S4: Support "known patterns" (e.g., "new screen" = 3 files: component, service, test)

### Epic 2: Phase Executor with Persistence
**Goal:** Run each phase, handle timeout gracefully, persist partial results.

**Stories:**
- E2S1: Spawn subagent with phase-specific context (files to create, existing codebase state)
- E2S2: Capture subagent output before timeout kills it
- E2S3: Store written files list in DB on timeout
- E2S4: Resume from last completed phase on restart

### Epic 3: Context Passing Between Phases
**Goal:** Each phase knows what previous phases created.

**Stories:**
- E3S1: Build "workspace snapshot" — list of files modified in this task
- E3S2: Generate "handoff letter" — summary of what was done, what's next
- E3S3: Include file contents of previously created files in next agent's prompt
- E3S4: Track imports/dependencies between created files

### Epic 4: Auto-Wiring Engine
**Goal:** Fix compilation errors automatically after all phases complete.

**Stories:**
- E4S1: Detect missing `mod` declarations and add them
- E4S2: Detect missing `use` imports and add them
- E4S3: Run `cargo check` after wiring, iterate on errors
- E4S4: Handle renames (e.g., agent A used `Task`, agent B used `SwarmTask`)

### Epic 5: Error Recovery + Retry
**Goal:** When a phase fails, retry with adjusted strategy.

**Stories:**
- E5S1: Store error output from failed phase
- E5S2: Check error journal for similar failures (pattern matching)
- E5S3: Retry with simplified scope (fewer files per phase)
- E5S4: Escalate to human after N retries

### Epic 6: Progress Dashboard Integration
**Goal:** Show phase-level progress in TUI dashboard.

**Stories:**
- E6S1: Add `phases` table to DB schema
- E6S2: Show phase timeline in Tasks tab (like GitHub progress bar)
- E6S3: Show current agent working on each phase
- E6S4: Show estimated completion time

---

## 7. Task Decomposition (Work Breakdown)

### Phase 1: Foundation (Must be sequential)
| Task | Depends On | Effort | Owner |
|------|-----------|--------|-------|
| T1.1: Create `phases` table schema | — | 1h | System Agent |
| T1.2: Implement TaskDecomposer (basic) | T1.1 | 2h | System Agent |
| T1.3: Implement StateManager (save/load phase state) | T1.1 | 2h | System Agent |
| T1.4: Wire into ExecutionLoop | T1.2, T1.3 | 1h | Code Agent |

### Phase 2: Phase Execution (Can parallel with Phase 3)
| Task | Depends On | Effort | Owner |
|------|-----------|--------|-------|
| T2.1: Implement PhaseExecutor with timeout capture | T1.4 | 3h | Code Agent |
| T2.2: Store "files created" on timeout | T2.1 | 1h | Code Agent |
| T2.3: Resume logic (read last completed phase) | T1.3 | 2h | Code Agent |

### Phase 3: Context Passing (Can parallel with Phase 2)
| Task | Depends On | Effort | Owner |
|------|-----------|--------|-------|
| T3.1: Workspace snapshot builder | — | 2h | Code Agent |
| T3.2: Handoff letter generator | T3.1 | 2h | Code Agent |
| T3.3: Include prior files in subagent prompt | T3.2 | 1h | Code Agent |

### Phase 4: Auto-Wiring (Depends on Phase 2 + 3)
| Task | Depends On | Effort | Owner |
|------|-----------|--------|-------|
| T4.1: Missing `mod` detector | — | 2h | Code Agent |
| T4.2: Missing `use` import detector | T4.1 | 2h | Code Agent |
| T4.3: `cargo check` iteration loop | T4.2 | 3h | Code Agent |
| T4.4: Rename conflict handler | T4.3 | 2h | Code Agent |

### Phase 5: Error Recovery (Depends on Phase 2)
| Task | Depends On | Effort | Owner |
|------|-----------|--------|-------|
| T5.1: Error output capture | T2.1 | 1h | Code Agent |
| T5.2: Error journal pattern matching | T5.1 | 2h | Code Agent |
| T5.3: Retry with simplified scope | T5.2 | 2h | Code Agent |
| T5.4: Human escalation after N retries | T5.3 | 1h | Code Agent |

### Phase 6: Dashboard (Depends on Phase 1)
| Task | Depends On | Effort | Owner |
|------|-----------|--------|-------|
| T6.1: Add phases table to DB | T1.1 | 1h | Code Agent |
| T6.2: Phase progress bar widget | T6.1 | 2h | Code Agent |
| T6.3: Phase timeline in Tasks tab | T6.2 | 2h | Code Agent |

---

## 8. Dependency Graph

```
                    ┌─────────────────┐
                    │  T1.1 Schema    │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
        ┌─────────┐    ┌─────────┐    ┌─────────┐
        │ T1.2    │    │ T1.3    │    │ T3.1    │
        │Decompose│    │StateMgr │    │Snapshot │
        └────┬────┘    └────┬────┘    └────┬────┘
             │              │              │
             └──────────────┼──────────────┘
                            │
                            ▼
                      ┌─────────┐
                      │ T1.4    │
                      │Wire Loop│
                      └────┬────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
        ┌─────────┐  ┌─────────┐  ┌─────────┐
        │ T2.1    │  │ T3.2    │  │ T6.1    │
        │Execute  │  │Handoff  │  │DB Phase │
        └────┬────┘  └────┬────┘  └────┬────┘
             │            │            │
             └────────────┼────────────┘
                          │
                          ▼
                    ┌─────────┐
                    │ T2.2    │
                    │Capture  │
                    └────┬────┘
                          │
             ┌────────────┼────────────┐
             │            │            │
             ▼            ▼            ▼
       ┌─────────┐  ┌─────────┐  ┌─────────┐
       │ T2.3    │  │ T5.1    │  │ T4.1    │
       │Resume   │  │Capture  │  │Mod      │
       └─────────┘  └────┬────┘  └────┬────┘
                         │            │
                         └────────────┼────────────┘
                                    │
                                    ▼
                              ┌─────────┐
                              │ T4.3    │
                              │Compile  │
                              │Loop     │
                              └─────────┘
```

---

## 9. Acceptance Criteria

### Feature: Task Decomposition
- [ ] AC1.1: A task with "create 5 files" is broken into 2-3 phases
- [ ] AC1.2: Each phase has `estimated_time < 120s` (2 min buffer before 2.5m timeout)
- [ ] AC1.3: Phases are stored in DB before execution begins
- [ ] AC1.4: Phase order respects file dependencies (e.g., model before controller)

### Feature: Phase Execution
- [ ] AC2.1: Phase starts, subagent receives correct context
- [ ] AC2.2: On timeout, partial files are saved and listed in DB
- [ ] AC2.3: On success, all files are committed to git (sandbox branch)
- [ ] AC2.4: Resume after restart reads last completed phase from DB

### Feature: Context Passing
- [ ] AC3.1: Phase N receives list of files created in phases 1..N-1
- [ ] AC3.2: Phase N receives "handoff letter" explaining prior work
- [ ] AC3.3: No agent recreates a file that already exists
- [ ] AC3.4: Imports from prior files are suggested in prompt

### Feature: Auto-Wiring
- [ ] AC4.1: After all phases, `cargo check` passes on first try (≥80% of tasks)
- [ ] AC4.2: Missing `mod` declarations are auto-added
- [ ] AC4.3: Missing `use` imports are auto-added (≥70% accuracy)
- [ ] AC4.4: Wiring failures are logged for human review

### Feature: Error Recovery
- [ ] AC5.1: Failed phase retries up to 3 times automatically
- [ ] AC5.2: Retry uses simplified scope (fewer files)
- [ ] AC5.3: After 3 failures, human is notified with full context
- [ ] AC5.4: Error patterns are stored in error journal

### Feature: Dashboard
- [ ] AC6.1: Tasks tab shows phase progress bar (e.g., ████░░░░ 50%)
- [ ] AC6.2: Phase names and statuses are visible
- [ ] AC6.3: Current agent working on each phase is shown
- [ ] AC6.4: Auto-refresh shows phase completion in real-time

---

## 10. Database Schema

### New Table: `phases`

```sql
CREATE TABLE phases (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    phase_number INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL CHECK(status IN ('pending', 'running', 'done', 'failed', 'retrying')),
    assigned_agent TEXT,
    files_expected TEXT, -- JSON array of file paths
    files_created TEXT,  -- JSON array of actual file paths
    started_at TEXT,
    completed_at TEXT,
    error_output TEXT,
    retry_count INTEGER DEFAULT 0,
    handoff_letter TEXT, -- Context for next phase
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);
```

### New Table: `phase_dependencies`

```sql
CREATE TABLE phase_dependencies (
    phase_id TEXT NOT NULL,
    depends_on_phase_id TEXT NOT NULL,
    PRIMARY KEY (phase_id, depends_on_phase_id),
    FOREIGN KEY (phase_id) REFERENCES phases(id),
    FOREIGN KEY (depends_on_phase_id) REFERENCES phases(id)
);
```

---

## 11. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| Subagent still times out even with 1-file phases | Medium | High | Retry with even smaller scope; add manual override |
| Auto-wiring breaks existing code | Medium | High | Git sandbox — always branch, never touch main |
| Context overflow (too many prior files in prompt) | High | Medium | Summarize prior files; include only relevant ones |
| Phase dependency detection is wrong | Medium | Medium | Default to conservative ordering; human can override |
| Cargo check loop infinite | Low | High | Max 5 iterations; then human escalation |

---

## 12. Success Metrics

| Metric | Baseline (v0.1.0) | Target (v0.2.0) |
|--------|-------------------|-----------------|
| Avg human interventions per 5-file task | 5 (every file) | 1 (just review) |
| Avg time to complete 5-file task | 30 min (hybrid) | 15 min (parallel) |
| Compilation success rate after swarm | 60% | 90% |
| Phase timeout rate | 100% (phases don't exist) | <20% |
| Human escalations per 10 tasks | N/A | <2 |

---

## 13. Test Plan

### Integration Test: 5-File Feature
1. Create task: "Add logging module with 5 files (config, logger, formatter, appender, tests)"
2. Start swarm
3. Verify phases created in DB
4. Verify each phase completes or retries
5. Verify all 5 files exist
6. Run `cargo check` — must pass
7. Verify git history shows 1 commit per phase

### Edge Case: Timeout Recovery
1. Create task with intentionally complex file
2. Phase 1 times out
3. Verify partial file saved
4. Verify retry triggered
5. Verify Phase 2 sees partial file in context

### Edge Case: Import Conflicts
1. Phase 1 creates `models.rs` with `struct Task`
2. Phase 2 creates `runner.rs` with `use crate::models::Task`
3. Phase 1 uses `Task`, Phase 2 uses `SwarmTask` (same concept, different name)
4. Verify auto-wirer detects conflict and unifies naming

---

*PRD v1.0 — May 9, 2026*
*Next: Epic breakdown → Task assignment → Parallel build*
