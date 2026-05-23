# OpenClaw Swarm - Phased Development Orchestration

**Status:** Design Document v1.2 (All decisions resolved)
**Date:** 2026-05-10
**Author:** Ayan (Queen's Architect)
**Research Sources:** E-mem (ICML 2026), Google ADK Context Engineering, LangMem, Anthropic "Building Effective Agents", Microsoft ISE "Multi-Agent Systems at Scale", AugmentCode "Multi-Agent Orchestration", Feature-Dev 7-Phase SOP, OpenClaw Swarm v0.2.0 codebase, AGENTS.md mental models

---

## 1. Executive Summary

The current swarm runs all agents in parallel on a single task. This works for small, well-defined builds, but falls apart for:
- Complex features requiring planning + architecture before code
- Multi-step workflows where downstream depends on upstream
- Builds that need human review at checkpoints
- Large epics that must be broken into stories

**This document defines a hierarchical, gated-phase orchestration system** where:
- The **Queen/Planning Agent** scopes work into **Epics → Stories → Tasks → Sub-tasks**
- Each **Story** proceeds through **phases** (Planning → Design → Implementation → Review → Ship)
- **Phase gates** require review before progression
- **Agents are selected per-phase**, not all-at-once
- The **mid-pane UI** shows active phases, agent conversations, and full history

---

## 2. The Full Story - Step by Step

### Act I: The Request Arrives

**You type:** "Build a Day Seal feature for MouminA. It should detect when a user completes all daily prayers + dhikr + Quran reading, then award a seal badge + bonus XP."

**Step 1: Intake & Triage**
- The Queen receives the request
- Queen spawns a **Planning Agent** (persona: "Ayan the Architect", personality: "analytical")
- Planning Agent analyzes: Is this an Epic or a Story?

**Sizing Decision Matrix:**
| Factor | Threshold |
|--------|-----------|
| Files touched | >5 files → Epic |
| DB tables modified | >2 tables → Epic |
| Cross-module? | Touches 2+ modules → Epic |
| New feature area? | Entirely new tab/screen → Epic |
| Estimated agent-hours | >30 min → Epic |

For Day Seal: Touches UserProvider, XP service, DB schema, UI badge. **→ Story** (not Epic).

**Step 2: Story Definition**
Planning Agent creates:
```
Story: "Implement Day Seal Feature"
├── Scope: Single story (5 files, 2 modules, ~20 min)
├── Parent Epic: (none - standalone story)
├── Phases:
│   1. Planning (Queen + Planning Agent)
│   2. Design (System Agent - architecture doc)
│   3. Implementation (Code Agents - parallel)
│   4. Review (Reviewer Agents - parallel)
│   5. Ship (Queen + Sandbox)
└── Estimated Duration: 15-20 minutes
```

The Planning Agent presents this plan to you in the mid-pane. **You approve.**

---

### Act II: Phase Execution

**Phase 1: Planning (Sequential, 1 Agent)**
- Agent: Planning Agent
- **Pre-step: Auto-ingest codebase via graphify** — maps existing files, conventions, patterns to knowledge graph. Planning Agent reads graph before scoping.
- Task: Finalize requirements, identify edge cases, list files to touch, define naming conventions
- Output: `plans/day-seal-plan.md` + phase breakdown + naming conventions artifact
- Gate: You review the plan. **Approve → Phase 2.**

**Phase 2: Design (Sequential, 1-2 Agents)**
- Agent: System Agent (architect)
- Task: Design DB schema changes, API contracts, component hierarchy
- Output: `designs/day-seal-design.md` + schema SQL + interface definitions
- Gate: You review design. **Approve → Phase 3.**
- *If design rejected:* Back to Phase 1 with feedback.

**Phase 3: Implementation (Parallel, N Agents)**
- Agents assigned by specialty:
  - Coder A: DB schema + migration
  - Coder B: XP service logic
  - Coder C: UI components (badge, trigger)
  - Coder D: Integration wiring (UserProvider day-change detection)
- Each agent works in parallel on their sub-task
- Swarm Bus carries letters between agents for coordination
- Output: Code commits in `swarm/build-day-seal/<id>/`
- Gate: Auto-check (compile + tests pass) + **your quick scan**

**Phase 4: Review (Parallel, 3 Agents)**
- Reviewer A (simplicity/DRY): "Is this the minimum code needed?"
- Reviewer B (bugs/correctness): "Run edge case analysis"
- Reviewer C (conventions): "Match existing MouminA patterns?"
- Output: Review report with findings + recommendations
- Gate: **You decide** what to fix. Fixes spawn mini-tasks → back to Phase 3 (surgical). **Approve → Phase 5.**

**Phase 5: Ship (Sequential, Queen + Sandbox)**
- Sandbox merges branch to main
- Git commit with agent-tagged message
- Graphify auto-runs: maps new concepts to knowledge graph
- Task status → `Merged`

---

### Act III: What You See in the UI

**Mid-Pane Layout (Active Story):**

```
┌─────────────────────────────────────────────────────────────┐
│  Story: Implement Day Seal Feature          [⏱️ 12m]       │
│  Status: Phase 3 - Implementation ████████░░ 80%           │
├─────────────────────────────────────────────────────────────┤
│  Phase Timeline:                                            │
│  ✅ Phase 1: Planning        (2m)   [view]                │
│  ✅ Phase 2: Design            (3m)   [view]                │
│  ▶️  Phase 3: Implementation   (7m)   ← ACTIVE            │
│  ⏸️ Phase 4: Review            (pending)                  │
│  ⏸️ Phase 5: Ship              (pending)                  │
├─────────────────────────────────────────────────────────────┤
│  Active Agents (4):                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│  │ 🤖 Coder A  │ │ 🤖 Coder B  │ │ 🤖 Coder C  │           │
│  │  db/schema  │ │  xp logic   │ │  UI badge   │           │
│  │  [live log] │ │  [live log] │ │  [live log] │           │
│  └─────────────┘ └─────────────┘ └─────────────┘           │
├─────────────────────────────────────────────────────────────┤
│  Activity Stream:                                           │
│  [07:51:23] Coder A → "Created migration v5_add_day_seal"  │
│  [07:51:45] Coder B → "Wrote calculateDaySeal() in xp.ts"  │
│  [07:52:01] Letter: Coder A → Coder D "Need user.id type"  │
│  [07:52:15] Coder C → "Badge component rendered"           │
│  [07:52:30] Auto-check: ✅ Compile OK, ⚠️ 1 test skip      │
├─────────────────────────────────────────────────────────────┤
│  [Approve Phase] [Request Changes] [Pause] [Abort]          │
└─────────────────────────────────────────────────────────────┘
```

**History View (Completed Story):**
- Full phase timeline with duration
- Every agent conversation (letters) preserved
- Every file change with diff
- Every decision and why it was made
- Links to design docs, plans, review reports
- Knowledge graph auto-generated: `Day Seal → XP Engine → UserProvider`

---

### Act IV: The Epic Scenario

**You type:** "Rebuild the entire Social tab. Friends, feed, messaging, notifications."

**Step 1: Sizing**
Planning Agent: "This touches 8+ files, 4 DB tables, 3 modules, new screens. **→ Epic.**"

**Step 2: Epic Breakdown**
```
Epic: "Social Tab Rebuild"
├── Story 1: "Friendship System"          (DB + API + UI)
├── Story 2: "Activity Feed"              (DB + API + UI)
├── Story 3: "Direct Messaging"           (DB + API + UI)
├── Story 4: "Social Notifications"       (integration)
└── Story 5: "Social Settings / Privacy"  (UI + logic)
```

**Step 3: Epic Dashboard**
```
┌─────────────────────────────────────────────────────────────┐
│  Epic: Social Tab Rebuild                    [⏱️ 2h 15m]  │
│  Progress: 2/5 Stories complete                             │
├─────────────────────────────────────────────────────────────┤
│  Story Board:                                               │
│  ┌────────────────┐ ┌────────────────┐ ┌────────────────┐  │
│  │ ✅ Friendship  │ │ ▶️ Activity    │ │ ⏸️ Messaging     │  │
│  │    System      │ │    Feed        │ │                │  │
│  │   [view]       │ │   [view live]  │ │   [pending]     │  │
│  └────────────────┘ └────────────────┘ └────────────────┘  │
│  ┌────────────────┐ ┌────────────────┐                    │
│  │ ⏸️ Notifications│ │ ⏸️ Settings    │                    │
│  │   [pending]     │ │   [pending]     │                    │
│  └────────────────┘ └────────────────┘                    │
└─────────────────────────────────────────────────────────────┘
```

Each story runs through its own 5 phases. Stories can be parallel (Story 1 + 2 simultaneously) or sequential (Story 4 depends on Story 1 completion).

---

## 3. Research-Backed Enhancements

### 3.1 Adaptive Re-planning (from Agentic Lybic)
Current swarm: static plan, execute once.
**Enhancement:** If a phase fails review or tests, don't just retry - **re-plan**. The Planning Agent re-evaluates with new context and may:
- Split a sub-task further
- Change agent assignments
- Add a new phase (e.g., "Spike" phase for unknown tech)

### 3.2 Topology Selection Per Phase (from AdaptOrch benchmark)
Not all phases should use parallel agents:

| Phase | Best Topology | Why |
|-------|--------------|-----|
| Planning | Sequential (1 agent) | Needs coherent thought chain |
| Design | Sequential → Parallel (2 agents debate) | Architecture benefits from parallel exploration |
| Implementation | Parallel (N agents) | Independent files/modules |
| Review | Parallel (3 agents) | Independent review dimensions |
| Ship | Sequential (Queen) | Single merge point |

The Queen selects topology per phase automatically.

### 3.3 Context Reset Between Phases — Tiered Context Briefing (APPROVED)

**Approach:** Compile a structured "Phase Briefing" — not full context reset, not new subagent spawn.

**Why:**
- E-mem (ICML 2026) shows that keeping uncompressed episodic chunks + selective activation beats compression
- Google ADK's "Context Engineering" treats context as a compiled view over rich state, not a mutable string buffer
- Our own `sessions_spawn` has a 2.5 min timeout — making it unsuitable for phase boundaries

**Implementation:**

```yaml
phase_briefing:
  meta:
    previous_phase: "Design"
    current_phase: "Implementation"
    story: "Implement Day Seal Feature"
    
  activated_episodes:
    - artifact: "designs/day-seal-design.md"
      type: "design_doc"
      summary: "calculateDaySeal() returns boolean, called from UserProvider"
    - artifact: "plans/day-seal-plan.md"
      type: "plan"
      summary: "Day Seal triggers on all prayers + dhikr + quran completion"
      
  current_task:
    description: "Write calculateDaySeal() in services/xp.ts"
    success_criteria: "Detects completion, awards 50 XP, updates streak"
    output_format: "TypeScript function + JSDoc + 3 edge case tests"
    
  boundary:
    dont_worry_about: ["UI badge (Coder C)", "DB migration (Coder A)"]
    ask_before: ["Changing API signatures", "Exceeding 100 lines"]
```

**How it works:**
1. Previous phase outputs are stored as artifacts in `artifacts/` table (uncompressed)
2. Agent receives: `EPISODE BRIEFING` + `CURRENT TASK` + `BOUNDARY`
3. Agent has full context of what matters, explicit knowledge of what doesn't
4. No spawn overhead, no context loss, no 2.5 min timeout risk

**Source:** E-mem paper, Google ADK Context Engineering, LangMem background memory manager

### 3.4 Semantic Agent Selection — Queen Knows Her Court (APPROVED)

**Approach:** Explicit capability declarations in persona YAML + symbolic keyword matching. NOT vector embeddings. NOT loading all 81 persona×personality combos.

**Why:**
- Microsoft ISE uses Azure AI Search + embeddings for agent selection — requires vector DB + API calls
- E-mem's Multi-Pathway Routing shows that **symbolic triggers** (keyword/entity matching) are fastest and cheapest
- With only 9 personas, vector search is overkill. Explicit YAML declarations are deterministic, explainable, and fast.

**Persona YAML Extension:**

```yaml
# personas/system_agent.yaml
name: "Ayan the Architect"
description: "Designs systems, schemas, and APIs"

capabilities:
  - architecture_design
  - schema_modeling
  - api_contracts
  - technology_evaluation
  - tradeoff_analysis

sample_tasks:
  - "Design database schema for new feature"
  - "Create API contract between frontend and backend"
  - "Evaluate tradeoffs between two approaches"

phases_i_work_on:
  - planning
  - design

phases_i_dont_work_on:
  - implementation    # "I architect, I don't code"
  - review            # "I design, I don't review my own designs"
  - ship              # "I design, I don't merge"
```

**Queen's Court Selection Algorithm:**

```python
def select_agents_for_phase(task_description, phase_name, available_personas):
    candidates = []
    for persona in available_personas:
        # Gate 1: Does this persona work on this phase?
        if phase_name not in persona.phases_i_work_on:
            continue
            
        # Gate 2: Symbolic trigger — keyword matching from capabilities
        task_keywords = extract_keywords(task_description)
        capability_score = sum(1 for cap in persona.capabilities if cap in task_keywords)
        
        # Gate 3: Task pattern matching
        task_match = any(sample in task_description for sample in persona.sample_tasks)
        
        if capability_score > 0 or task_match:
            candidates.append((persona, capability_score + (2 if task_match else 0)))
    
    # Sort by relevance score, take top N for phase topology
    candidates.sort(key=lambda x: x[1], reverse=True)
    max_agents = get_topology_max_agents(phase_name)  # Planning=1, Impl=4, etc.
    return [c[0] for c in candidates[:max_agents]]
```

**Benefits:**
- **Deterministic:** Same task → same agents every time
- **Explainable:** Queen can say "I chose System Agent because architecture_design matched"
- **Fast:** String matching, not vector computation
- **Zero dependencies:** No OpenAI API, no vector DB, no Azure Search

**Example:**
- Task: "Design DB schema for Day Seal" + Phase: "design" → System Agent (architecture, schema, planning, design)
- Task: "Write calculateDaySeal()" + Phase: "implementation" → Coder A (db), Coder B (logic) — System Agent excluded by `phases_i_dont_work_on`

### 3.5 Phase Gate Patterns (from Feature-Dev Skill)
Integrate the 7-Phase SOP directly:
1. Discovery → Phase 1 (Planning)
2. Codebase Exploration → Phase 1.5 (auto-run parallel explorer agents)
3. Clarifying Questions → Phase 1 gate (must answer before Phase 2)
4. Architecture Design → Phase 2 (Design)
5. Implementation → Phase 3
6. Quality Review → Phase 4
7. Summary → Phase 5 (Ship) + auto-generated summary

### 3.6 Recovery Primitives (from GraSP paper)
When a phase fails:
- **Rebind**: Update arguments, retry same agent
- **InsertPrereq**: Add a missing prerequisite phase
- **Substitute**: Swap failing agent for another persona
- **Rewire**: Change task dependencies
- **Bypass**: Skip if downstream already satisfied
- **Escalate**: Queen intervenes manually

---

## 3.7 Agent Context Protocol (APPROVED)

Every agent, at every phase, receives a structured context header. This implements the **Circle of Competence** mental model directly into the agent prompt.

**Based on:** Google ADK tiered context + Anthropic augmented LLM + our own Circle of Competence mental model

### The 5-Section Context Header

```yaml
agent_context:
  goal:
    epic: "Social Tab Rebuild"
    story: "Implement Day Seal Feature"
    task: "Write calculateDaySeal() in services/xp.ts"
    success_criteria: "Function detects completion, awards 50 XP, updates streak"
    
  circle_of_competence:
    knows:
      - "MouminA XP service architecture"
      - "SQLite schema v4"
      - "User type uses `id` not `userId`"
    doesnt_know:
      - "UI component design (handled by Coder C)"
      - "DB migration syntax (handled by Coder A)"
      - "Exact day-change detection logic (handled by Coder D)"
    will_ask:
      - "Clarify ambiguous requirements before coding"
      - "Flag if success criteria are underspecified"
      - "Request review if solution exceeds 100 lines"
      
  role_in_this_phase:
    phase: "Implementation"
    topology: "parallel"
    peers:
      - persona: "coder_a"
        doing: "DB migration"
      - persona: "coder_c"
        doing: "UI badge component"
    reports_to: "coordinator"
    
  expectations:
    output: "TypeScript function in services/xp.ts"
    tests: "At least 3 edge cases tested"
    documentation: "JSDoc with params/returns"
    commit_message: "feat(xp): add day seal calculation"
    naming_conventions:
      functions: "snake_case"
      tables: "snake_case, prefix with module"
      components: "PascalCase"
      variables: "camelCase"
    undefined_items_policy: "ASK_LEADER_BEFORE_DEFINING"
    
  relevant_history:
    - phase: "Planning"
      artifact: "plans/day-seal-plan.md"
      summary: "Day Seal triggers on all prayers + dhikr + quran completion"
    - phase: "Design"
      artifact: "designs/day-seal-design.md"
      summary: "calculateDaySeal() returns boolean, called from UserProvider day-change hook"
```

### Why This Works

| Section | Purpose | Prevents |
|---------|---------|----------|
| **Goal** | Agent knows full chain (epic→story→task) | Siloed decisions that break higher purpose |
| **Circle of Competence** | Explicitly states what it doesn't know | Silent assumptions, overreach, wrong fixes |
| **Role** | Knows peers, topology, reporting line | Duplicate work, conflicting implementations |
| **Expectations** | Concrete output spec + naming conventions | Ambiguity, missing tests, wrong format, inconsistent naming |
| **Relevant History** | Activated episodic chunks only | Context overload, irrelevant noise |

### Implementation

The **Coordinator** compiles this header before invoking any agent. It reads:
1. Story metadata from `tasks` table
2. Phase artifacts from `artifacts` table
3. Peer assignments from `phase_assignments` table
4. Persona capabilities from YAML

Then prepends the compiled YAML to the agent's task prompt.

**Also reads naming conventions from Planning phase artifact** (if exists) to enforce `snake_case` vs `camelCase` consistency across all agents in a story.

### Conflict Resolution Rules (When Agents Disagree)

Based on our mental models from AGENTS.md:

| Principle | Rule | When to Apply |
|-----------|------|---------------|
| **Falsifiability** | "Which choice is easier to test/disprove?" | When comparing technical approaches |
| **First Principles** | "Which is closer to fundamental truth?" | When making architecture decisions |
| **Occam's Razor** | "Fewer assumptions wins" | When debating complexity tradeoffs |
| **Simplicity** | "200 lines → 50 lines?" | When code approaches diverge |
| **Inversion** | "What's the minimum that definitely works?" | When scope is uncertain |
| **Parallel Comparison** | "Build both, stress test, measure, pick winner" | When agents disagree and both seem equal |
| **Hanlon's Razor** | "Bugs are oversight, not malice. Be kind." | When reviewing another agent's work |

**Escalation chain:**
1. Agents disagree → apply principles above
2. Still deadlocked → parallel build + measure
3. Still deadlocked → ask Coordinator
4. Coordinator can't decide → **Queen (user) has veto authority**

**Result:** Every agent starts with full situational awareness, explicit boundaries, zero ambiguity about what it's supposed to do, and clear rules for when it doesn't know something.

---

## 4. Requirements

### 4.1 Functional Requirements

**FR-1: Hierarchical Task Model**
- Support 4 levels: Epic → Story → Task → Sub-task
- Parent-child relationships with dependency tracking
- Epic aggregates progress from child stories

**FR-2: Phase System**
- Each Story has 5 default phases: Planning, Design, Implementation, Review, Ship
- Phases are configurable per story type
- Phase gates require explicit approval (auto or human)
- Phase outputs are persisted as artifacts (docs, code, reviews)

**FR-3: Planning Agent**
- Dedicated persona for requirement analysis and scoping
- Outputs: scope decision (epic vs story), phase plan, agent assignments
- Presents plan to user for approval before execution

**FR-4: Adaptive Agent Selection (Queen Knows Her Court)**
- Per-phase agent pool selection via **persona capability YAML + symbolic keyword matching**
- Persona YAML declares: `capabilities`, `sample_tasks`, `phases_i_work_on`, `phases_i_dont_work_on`
- Selection algorithm: Phase gate → Symbolic keyword match → Task pattern match → Score sort
- Agent factory pattern: instantiate agents by persona + compiled context briefing
- Reassignment supported mid-phase (Queen command)

**FR-5: Review & Gate System**
- Post-implementation: parallel review agents (simplicity, bugs, conventions)
- Review report with severity scoring
- User can: Approve, Request Changes (with feedback), or Reject (abort)
- Request Changes spawns surgical fix tasks → back to Implementation

**FR-6: Activity Stream & History**
- All agent actions logged: letters, file writes, commits, test runs
- Activity stream is real-time (SSE/WebSocket)
- Completed stories retain full history: phases, conversations, diffs, decisions
- History is searchable and exportable

**FR-7: Mid-Pane UI**
- Active story/phase shown in center panel
- Agent cards with live status and logs
- Phase timeline with progress indicators
- Activity stream feed
- Action buttons: Approve, Request Changes, Pause, Abort

**FR-8: Epic Dashboard**
- Story board view (Kanban-style lanes)
- Dependency graph between stories
- Epic-level progress aggregation
- Click story → drill into its phase view

### 4.2 Non-Functional Requirements

**NFR-1: Persistence**
- All state in SQLite: tasks, phases, steps, activities, letters
- Resume after crash: restore active tasks to last known state
- History never deleted (append-only audit log)

**NFR-2: Real-Time Updates**
- Activity stream pushes via SSE (already implemented in web dashboard)
- UI updates within 2 seconds of agent action
- No polling loops

**NFR-3: Token Efficiency**
- **Tiered phase briefing** (activated episodes, not full history dump)
- **Queen Knows Her Court** (capability matching, only load relevant agents)
- Phase artifacts summarized for downstream consumption
- Context compiled per phase, not accumulated forever

**NFR-4: Graceful Degradation**
- If subagent spawn fails → fallback to main session execution
- If review agents timeout → flag for manual review
- If phase gate not answered in 10 min → auto-pause, notify user

---

## 5. Architecture Design

### 5.1 Database Schema Extensions

```sql
-- Task hierarchy (extends existing tasks table)
ALTER TABLE tasks ADD COLUMN parent_id TEXT REFERENCES tasks(id);
ALTER TABLE tasks ADD COLUMN task_level TEXT CHECK(task_level IN ('epic','story','task','subtask')) DEFAULT 'task';
ALTER TABLE tasks ADD COLUMN story_type TEXT DEFAULT 'sdlc_feature'; -- epic types too

-- NEW: Phases table
CREATE TABLE story_phases (
    id TEXT PRIMARY KEY,
    story_id TEXT NOT NULL REFERENCES tasks(id),
    phase_number INTEGER NOT NULL,
    phase_name TEXT NOT NULL, -- 'planning', 'design', 'implementation', 'review', 'ship'
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','blocked','reviewing','approved','rejected','skipped')),
    topology TEXT DEFAULT 'sequential' CHECK(topology IN ('sequential','parallel','hybrid')),
    started_at TEXT,
    completed_at TEXT,
    approved_by TEXT, -- 'auto', 'user', 'queen'
    approval_note TEXT,
    artifact_path TEXT, -- path to phase output (plan.md, design.md, review.json)
    UNIQUE(story_id, phase_number)
);

-- NEW: Phase assignments (which agents work on which phase)
CREATE TABLE phase_assignments (
    id TEXT PRIMARY KEY,
    phase_id TEXT NOT NULL REFERENCES story_phases(id),
    persona_id TEXT NOT NULL,
    personality_id TEXT NOT NULL,
    sub_task_description TEXT,
    status TEXT DEFAULT 'pending' CHECK(status IN ('pending','running','completed','failed')),
    assigned_at TEXT,
    completed_at TEXT,
    result_summary TEXT
);

-- NEW: Activity log (unified event stream)
CREATE TABLE activity_log (
    id TEXT PRIMARY KEY,
    story_id TEXT REFERENCES tasks(id),
    phase_id TEXT REFERENCES story_phases(id),
    actor_type TEXT NOT NULL CHECK(actor_type IN ('agent','user','system','queen')),
    actor_id TEXT,
    action_type TEXT NOT NULL CHECK(action_type IN ('phase_start','phase_complete','agent_start','agent_complete','letter_send','file_write','commit','test_run','review_submit','user_approve','user_reject','user_pause','error','replan')),
    payload TEXT, -- JSON: file path, letter content, test results, etc.
    timestamp TEXT DEFAULT CURRENT_TIMESTAMP
);

-- NEW: Artifacts table (phase outputs)
CREATE TABLE artifacts (
    id TEXT PRIMARY KEY,
    story_id TEXT REFERENCES tasks(id),
    phase_id TEXT REFERENCES story_phases(id),
    artifact_type TEXT NOT NULL CHECK(artifact_type IN ('plan','design','code','review','summary','test_report')),
    file_path TEXT NOT NULL,
    created_at TEXT,
    summary TEXT -- auto-generated 1-paragraph summary
);

-- NEW: Phase metrics (wall-clock time + token usage)
CREATE TABLE phase_metrics (
    id TEXT PRIMARY KEY,
    phase_id TEXT NOT NULL REFERENCES story_phases(id),
    wall_clock_seconds REAL, -- time from start to completion
    tokens_input INTEGER,     -- where CLI exposes this
    tokens_output INTEGER,    -- where CLI exposes this
    tokens_total INTEGER,     -- computed
    agent_invocations INTEGER, -- how many times this phase was retried
    created_at TEXT
);

-- NEW: Story dependencies (for epics)
CREATE TABLE story_dependencies (
    story_id TEXT NOT NULL REFERENCES tasks(id),
    depends_on_story_id TEXT NOT NULL REFERENCES tasks(id),
    dependency_type TEXT DEFAULT 'hard' CHECK(dependency_type IN ('hard','soft')),
    PRIMARY KEY(story_id, depends_on_story_id)
);
```

### 5.2 State Machine Extensions

**Task FSM (per story):**
```
Queued → Planning → Designing → Implementing → Reviewing → ReadyToMerge → Merging → Merged
    ↓         ↓           ↓              ↓              ↓
  Blocked  Blocked    Blocked       Blocked       Blocked
    ↓         ↓           ↓              ↓              ↓
  Failed   Failed     Failed        Failed        Failed
```

**Phase FSM (within each story phase):**
```
Pending → Running → Reviewing → Approved → (next phase)
    ↓         ↓          ↓
  Skipped  Blocked   Rejected → Running (replan/fix)
    ↓         ↓
  (bypass)  Failed
```

### 5.3 Component Interactions

```
User Request
    ↓
[Queen] - decides: Planning Agent needed?
    ↓
[Planning Agent] - analyzes, scopes, creates plan
    ↓
[User] - approves plan (mid-pane UI)
    ↓
[Queen] - creates Story + Phases in DB
    ↓
For each Phase:
    [Queen] - selects topology + agents (semantic retrieval)
        ↓
    [Coordinator] - executes phase with selected topology
        ↓
    [Agents] - work in parallel or sequential per topology
        ↓
    [Swarm Bus] - carries inter-agent letters
        ↓
    [Activity Logger] - records every action to DB
        ↓
    [Phase Gate] - auto-check + user review
        ↓
    [If rejected] → [Replan Agent] or [Surgical Fix] → back
    [If approved] → next phase
    ↓
[Ship Phase] - Sandbox merge, git commit, graphify
    ↓
[Summary Agent] - auto-generates completion report
```

### 5.4 New Components Needed

| Component | Purpose | Location |
|-----------|---------|----------|
| `PlanningAgent` | Scoping, phase planning, agent selection | `src/planning/` |
| `PhaseManager` | Phase lifecycle, gates, transitions | `src/phases/` |
| `ActivityLogger` | Unified event stream to DB | `src/activity/` |
| `ArtifactStore` | Phase output persistence + summarization | `src/artifacts/` |
| `ReviewAgent` | Parallel code review (3 dimensions) | `src/review/` |
| `ReplanAgent` | Adaptive re-planning on failure | `src/replan/` |
| `EpicDashboard` | Web UI epic/story board | `src/web_dashboard/epic.rs` |
| `PhasePane` | Web UI phase timeline + agent cards | `src/web_dashboard/phase.rs` |
| `ActivityStream` | Web UI real-time feed | `src/web_dashboard/activity.rs` |
| `PhaseMetrics` | Wall-clock + token tracking per phase | `src/metrics.rs` |

---

## 6. UI/UX Design

### 6.1 Mid-Pane - Active Phase View

**Layout (3 columns on desktop, stacked on mobile):**

```
┌──────────────┬──────────────────────────┬──────────────┐
│  LEFT        │  CENTER (Mid-Pane)       │  RIGHT       │
│  Story Tree  │  Phase Timeline          │  Agent Cards  │
│              │  + Activity Stream       │  + Status     │
│  - Epic X    │                          │              │
│    - Story 1 │  ┌────────────────────┐  │  ┌────────┐  │
│      ✅ Done │  │ Phase 3: Impl      │  │  │Coder A │  │
│    - Story 2 │  │ ████████░░ 80%     │  │  │🟢 Live │  │
│      ▶️ Active│  │ [view plan]        │  │  │db done │  │
│      - Task A│  └────────────────────┘  │  └────────┘  │
│      - Task B│  [Activity Stream]       │  ┌────────┐  │
│  - Epic Y    │  07:52 Coder B: wrote   │  │Coder B │  │
│    (pending) │  calculateDaySeal()     │  │🟡 Busy │  │
│              │  07:53 Test: ✅ pass    │  │xp svc  │  │
│              │  07:54 Letter: A→D      │  └────────┘  │
│              │                         │              │
│              │  [Approve] [Changes]     │              │
│              │  [Pause]  [Abort]       │              │
└──────────────┴──────────────────────────┴──────────────┘
```

### 6.2 History View - Completed Story

```
┌─────────────────────────────────────────────────────────┐
│  Story: Implement Day Seal        ✅ Completed 2026-05-10│
├─────────────────────────────────────────────────────────┤
│  Timeline:                                              │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  📋 Phase 1: Planning (2m)                              │
│     ├─ Agent: Planning Agent                           │
│     ├─ Output: plans/day-seal-plan.md                   │
│     └─ [view artifact]                                  │
│                                                         │
│  🏗️ Phase 2: Design (3m)                                │
│     ├─ Agents: System Agent                              │
│     ├─ Output: designs/day-seal-design.md               │
│     └─ [view artifact]                                  │
│                                                         │
│  💻 Phase 3: Implementation (7m)                        │
│     ├─ Agents: Coder A, B, C, D (parallel)            │
│     ├─ Commits: 4 commits                               │
│     └─ [view commits] [view diffs]                      │
│                                                         │
│  🔍 Phase 4: Review (2m)                                │
│     ├─ Agents: Reviewer A, B, C                       │
│     ├─ Report: 2 warnings, 0 errors                     │
│     └─ [view report]                                    │
│                                                         │
│  🚀 Phase 5: Ship (1m)                                  │
│     ├─ Merged to main                                 │
│     ├─ Commit: `feat: day seal detection + badge`     │
│     └─ [view in GitHub]                                 │
├─────────────────────────────────────────────────────────┤
│  Agent Conversations:                                   │
│  [07:52:01] Coder A → Coder D: "Need to clarify        │
│             user.id vs userId - which does UserProvider  │
│             use?"                                       │
│  [07:52:15] Coder D → Coder A: "user.id - confirmed."  │
│  ...                                                    │
├─────────────────────────────────────────────────────────┤
│  Knowledge Graph:                                       │
│  Day Seal → XP Engine → UserProvider → PrayerTimes      │
│  [view graph]                                           │
└─────────────────────────────────────────────────────────┘
```

### 6.3 Epic Dashboard

```
┌─────────────────────────────────────────────────────────┐
│  Epic: Social Tab Rebuild              [⏱️ 2h 15m]     │
│  Progress: ████████████████░░░░░░ 40% (2/5 stories)    │
├─────────────────────────────────────────────────────────┤
│  Story Board:                                           │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐    │
│  │ ✅ Story 1   │ │ ▶️ Story 2   │ │ ⏸️ Story 3   │    │
│  │ Friendship   │ │ Activity Feed│ │ Messaging    │    │
│  │   [12m]      │ │   [45m]      │ │   [pending]  │    │
│  │   [view]     │ │   [view live]│ │              │    │
│  └──────────────┘ └──────────────┘ └──────────────┘    │
│  ┌──────────────┐ ┌──────────────┐                      │
│  │ ⏸️ Story 4   │ │ ⏸️ Story 5   │                      │
│  │ Notifications│ │ Settings     │                      │
│  │   [pending]  │ │   [pending]  │                      │
│  └──────────────┘ └──────────────┘                      │
├─────────────────────────────────────────────────────────┤
│  Dependencies: Story 4 depends on Story 1                 │
│  [view dependency graph]                                │
└─────────────────────────────────────────────────────────┘
```

---

## 7. Implementation Roadmap

### Phase A: Foundation (Week 1)
1. Extend DB schema (hierarchy, phases, activity, artifacts, dependencies)
2. Extend Task FSM with phase states
3. Build `PhaseManager` - phase lifecycle + gates
4. Build `ActivityLogger` - unified event stream
5. Update web dashboard API endpoints for phases + activity

### Phase B: Planning & Orchestration (Week 1-2)
6. Build `PlanningAgent` - requirement analysis + scoping
7. Integrate semantic agent selection (narrow agent pool per phase)
8. Build topology selection logic (sequential/parallel/hybrid)
9. Context reset between phases (fresh briefing)

### Phase C: Review & Quality (Week 2)
10. Build `ReviewAgent` - 3-dimension parallel review
11. Build phase gate UI (approve/reject/request changes)
12. Build replan logic (InsertPrereq, Substitute, Rewire)
13. Build artifact store (plan.md, design.md, review.json persistence)

### Phase D: UI/UX (Week 2-3)
14. Build Epic Dashboard (story board, dependency graph)
15. Build Phase Pane (timeline, agent cards, progress)
16. Build Activity Stream (real-time feed, searchable history)
17. Build History View (completed story full timeline)

### Phase E: Integration & Polish (Week 3)
18. End-to-end test: small story (Day Seal)
19. End-to-end test: epic (Social Tab Rebuild mock)
20. Error handling: graceful degradation paths
21. Documentation + skill packaging

---

## 8. Comparison: Before vs After

| Aspect | Current Swarm v0.2 | Phased Orchestration v1.0 |
|--------|-------------------|---------------------------|
| Task model | Flat (single task) | Hierarchical (Epic→Story→Task→Sub-task) |
| Execution | All agents parallel | Per-phase topology (seq/parallel/hybrid) |
| Planning | Queen auto-assigns | Planning Agent analyzes + you approve |
| Review | None (ship when done) | 3-dimension parallel review + gates |
| History | Task steps in DB | Full phase timeline + artifacts + conversations |
| UI | Task list + status | Epic board + phase pane + activity stream |
| Recovery | Retry same agent | Replan, substitute, rewire, bypass |
| Context | Accumulates forever | **Tiered phase briefing** (activated episodes) |
| Token use | All agents every time | **Queen Knows Her Court** (capability matching) |
| Agent context | Raw task description | **5-section YAML header** (Goal, CoC, Role, Expectations, History) |

---

## 9. Open Questions (Updated Post-Research)

### ✅ Approved Decisions

| # | Question | Decision | Rationale |
|---|----------|----------|-----------|
| 6 | **Context reset mechanism?** | **Tiered Phase Briefing** | E-mem + Google ADK research. Spawn new subagents is broken (2.5 min timeout). Full reset loses design decisions. |
| 7 | **Agent context protocol?** | **5-Section YAML Header** | Google ADK tiered context + Circle of Competence mental model. Every agent gets: Goal, Circle of Competence, Role, Expectations, Relevant History. |
| 8 | **Semantic selection?** | **Queen Knows Her Court** | Capability YAML + symbolic keyword matching. Deterministic, explainable, fast, zero API dependencies. Not vector search. |
| 9 | **Parallel stories naming?** | **Follow the plan. Ask leader before defining.** | Planning phase outputs naming conventions artifact. Agents must use `new_function`, not `newFunction`. If undefined, ask Coordinator/Queen. |
| 10 | **Agent pool size?** | **Expand personas.** | 9 → 12+ personas with specialized capabilities. See expanded persona pool below. |
| 11 | **Cost tracking?** | **Both tokens + wall-clock.** | Track wall-clock time (easy, timestamps). Track tokens where CLI exposes them. Store in `phase_metrics` table. |
| 12 | **Codebase auto-ingest?** | **Use existing graphify.** | Graphify already maps codebases to knowledge graphs (809 concepts). Auto-run before Planning phase. Not a "bridge" — just existing pipeline. |

### Expanded Persona Pool (12+ Personas)

| Persona | Capabilities | Phases |
|---------|-------------|--------|
| **Queen** | Orchestration, final decisions, veto authority | All (supervisor) |
| **Planning Agent** | Scoping, sizing, phase planning | Planning |
| **System Agent (Ayan)** | Architecture, schemas, APIs, tradeoffs | Planning, Design |
| **Database Architect** | Schema design, migrations, query optimization | Design, Implementation |
| **API Designer** | REST/GraphQL contracts, OpenAPI specs | Design |
| **Coder A (Backend)** | Business logic, services, data layer | Implementation |
| **Coder B (Frontend)** | React Native, UI components, hooks | Implementation |
| **Security Reviewer** | Auth, injection, secrets, OWASP | Review |
| **Performance Auditor** | N+1, query optimization, bundle size | Review |
| **Test Engineer** | Unit tests, integration tests, edge cases | Implementation, Review |
| **DevOps / CI-CD** | Docker, GitHub Actions, deployment | Ship |
| **Integration Specialist** | Third-party APIs, webhooks, adapters | Implementation |
| **Documentation Writer** | README, API docs, inline docs | Implementation, Ship |

Each persona YAML includes: `capabilities`, `sample_tasks`, `phases_i_work_on`, `phases_i_dont_work_on`, `naming_conventions_i_enforce`.

### ✅ All Decisions Resolved

All open questions are now resolved. Ready to build.

---

## 10. Next Steps

### ✅ All Decisions Resolved (v1.2)

| # | Decision | Status |
|---|----------|--------|
| 1 | Context reset mechanism | ✅ Tiered Phase Briefing |
| 2 | Agent context protocol | ✅ 5-Section YAML Header |
| 3 | Semantic selection | ✅ Queen Knows Her Court |
| 4 | Parallel stories naming | ✅ Ask leader + follow plan |
| 5 | Agent pool size | ✅ 12+ personas |
| 6 | Cost tracking | ✅ Tokens + wall-clock |
| 7 | Codebase auto-ingest | ✅ Use existing graphify |
| 8 | Conflict resolution | ✅ Principles + parallel comparison |

### 📚 Books Ready
- Kleppmann (systems-design) — read Ch 1,3,7 before Phase A
- Wooldridge (multi-agent-systems) — read Ch 8,9 before Phase B

### 🚀 Start Phase A: Foundation

| # | Task | File(s) | Effort |
|---|------|---------|--------|
| A1 | Add hierarchy columns to `tasks` table | `src/db/schema.sql` + migration | 30 min |
| A2 | Create `story_phases` table | `src/db/schema.sql` | 20 min |
| A3 | Create `phase_assignments` table | `src/db/schema.sql` | 15 min |
| A4 | Create `activity_log` table | `src/db/schema.sql` | 15 min |
| A5 | Create `artifacts` table | `src/db/schema.sql` | 15 min |
| A6 | Create `story_dependencies` table | `src/db/schema.sql` | 10 min |
| A7 | Create `phase_metrics` table | `src/db/schema.sql` | 10 min |
| A8 | Extend Task FSM with phase states | `src/task_fsm.rs` | 45 min |
| A9 | Build `PhaseManager` — phase lifecycle | `src/phases/manager.rs` | 60 min |
| A10 | Build `ActivityLogger` — event stream | `src/activity/logger.rs` | 45 min |
| A11 | Update web dashboard API — phases | `src/web_dashboard.rs` | 60 min |

**Total Phase A: ~6.5 hours** (2-3 sessions)

**Ready to start A1? Or do you want to review the updated doc first?**

---

*Document written by Ayan, Queen's Architect. Research synthesized from Anthropic, Microsoft ISE, AugmentCode, and 8-book swarm knowledge base.*
