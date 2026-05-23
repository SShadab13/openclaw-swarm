# Research Report: Context Reset, Agent Memory & Semantic Selection

**Date:** 2026-05-10  
**Researcher:** Ayan (Queen's Court)  
**Scope:** What industry actually does for our 3 identified gaps  

---

## Executive Summary

I researched what the industry actually does for:
1. **Context reset between phases** — E-mem (ICML 2026), Google ADK Context Engineering, LangMem
2. **Agent context protocol** — Google ADK tiered context, Anthropic's augmented LLM pattern
3. **Semantic agent selection** — Microsoft ISE semantic cache, E-mem multi-pathway routing

Key finding: **We don't need vector embeddings for agent selection.** The Queen knowing her court (persona YAMLs with explicit capabilities) is sufficient. For context reset, the industry uses **tiered context** — not full reset, but scoped briefing.

---

## 1. Context Reset Between Phases — What Industry Does

### E-mem (ICML 2026 — Accepted)

**Paper:** *E-mem: Multi-agent based Episodic Context Reconstruction for LLM Agent Memory*  
**GitHub:** `github.com/dog-last/E-mem`

**What it does:**
- Instead of compressing memory into embeddings/graphs, keeps **uncompressed episodic chunks**
- **Master Agent** orchestrates global planning, decides which assistants to activate
- **Assistant Agents** maintain uncompressed memory segments and do **local reasoning within activated segments**
- **Multi-Pathway Routing** combines three signals:
  1. Global Alignment (summary-based intent filtering)
  2. Semantic Association (vector similarity)
  3. Symbolic Trigger (keyword/entity matching)

**Relevance to us:**
- For phase transitions, we don't need full context reset. We need **selective activation** — brief the agent with only the relevant episodic chunks (design doc, requirements, previous phase output)
- The "fresh briefing" I described is essentially this: give the agent the artifact from the previous phase as its "activated segment"

**Key quote from industry analysis:**
> "The most realistic path is hybrid. Use compressed indices for cold or bulk search. When a task needs deep, sequential reasoning and the cost justifies it, reconstruct episodic context from uncompressed segments and run assistant-level inference."

**Application to OpenClaw Swarm:**
- Don't spawn new subagents (2.5 min timeout bug becomes a feature)
- Instead, **prepend a structured briefing** to each phase's prompt
- Store phase artifacts as uncompressed episodic chunks in `artifacts/` table
- Agent receives: `EPISODE BRIEFING` + `CURRENT TASK` + `RELEVANT HISTORY`

### Google ADK — Context Engineering

**Source:** `developers.googleblog.com/architecting-efficient-context-aware-multi-agent-framework-for-production/`

**What Google does:**
- **Tiered Context Architecture:**
  - **Agent-level context:** Role, tools, instructions (static YAML — our personas)
  - **Session-level context:** Conversation history, shared state (our Swarm Bus + DB)
  - **Callback-level context:** Runtime event data (our task steps)
- **Context Scoping:** Each agent only sees what it needs
- **No full resets** — instead, they **compile a view** over rich state

**Key insight:**
> "ADK treats context not as a mutable string buffer but as a compiled view over rich state."

**Application to us:**
- Our personas YAML = Agent-level context (already exists)
- Our DB + Swarm Bus = Session-level context (already exists)
- Our task steps + artifacts = Callback-level context (need to build)
- Between phases, we **compile a new view** — not reset everything

### LangMem (LangChain)

**What it does:**
- **Core memory API** — works with any storage
- **Hot path memory** — agents record/search during active conversations
- **Background memory manager** — auto-extracts, consolidates, updates knowledge
- **Native LangGraph integration**

**Relevance:**
- The "background memory manager" is what we need for our **ArtifactStore**
- Auto-summarize phase outputs, store as episodic memory
- Agents query: "what happened in the Design phase?" → get summarized artifact

---

## 2. Agent Context Protocol — What Each Agent Should Know

Based on Google ADK's tiered context + Anthropic's augmented LLM pattern + our Circle of Competence mental model:

### The Agent Context Briefing (Mandatory Header)

Every agent, at every phase, receives this structure:

```yaml
agent_context:
  goal:
    epic: "Social Tab Rebuild"
    story: "Implement Day Seal Feature"
    task: "Write calculateDaySeal() in xp.ts"
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
  
  relevant_history:
    - phase: "Planning"
      artifact: "plans/day-seal-plan.md"
      summary: "Day Seal triggers on all prayers + dhikr + quran completion"
    - phase: "Design"
      artifact: "designs/day-seal-design.md"
      summary: "calculateDaySeal() returns boolean, called from UserProvider day-change hook"
```

### Why This Works

1. **Goal** — Agent knows the full chain (epic → story → task) so decisions align with higher purpose
2. **Circle of Competence** — Agent explicitly states what it doesn't know, preventing silent assumptions
3. **Role** — Agent knows who it's working with, preventing duplicate work
4. **Expectations** — Concrete output spec, no ambiguity
5. **Relevant History** — Not full context dump, just activated episodic chunks

---

## 3. Semantic Agent Selection — "Queen Knows Her Court"

### What Microsoft ISE Does

**Source:** Microsoft ISE blog on multi-agent systems at scale

- They use **Azure AI Search** with embeddings for agent selection
- Each agent has: name + 5+ sample utterances embedded
- Query embedded → retrieve top-k matching agents by similarity

**The problem:** This requires a vector database + embedding model + search index. Overkill for 9 personas.

### What E-mem Does

Multi-Pathway Routing:
1. **Symbolic Trigger** — keyword/entity matching (fastest, cheapest)
2. **Semantic Association** — vector similarity (medium cost)
3. **Global Alignment** — summary-based intent filtering (most accurate)

### What We Should Do — "Queen Knows Her Court"

Since we have only 9 personas, we don't need vector search. We need **explicit capability declarations** in persona YAMLs.

```yaml
# personas/system_agent.yaml
capabilities:
  - architecture_design
  - schema_modeling
  - api_contracts
  - technology_evaluation
  
sample_tasks:
  - "Design database schema for new feature"
  - "Create API contract between frontend and backend"
  - "Evaluate tradeoffs between two approaches"
  
phases_i_work_on:
  - planning
  - design
  
phases_i_dont_work_on:
  - implementation  # "I architect, I don't code"
  - review         # "I design, I don't review my own designs"
```

**Selection Algorithm (Queen's Court Selection):**

```python
def select_agents_for_phase(task_description, phase_name, available_personas):
    selected = []
    for persona in available_personas:
        # Check 1: Does this persona work on this phase?
        if phase_name not in persona.phases_i_work_on:
            continue
            
        # Check 2: Symbolic trigger — keyword matching
        keywords = extract_keywords(task_description)
        capability_match = any(cap in keywords for cap in persona.capabilities)
        
        # Check 3: Task pattern matching
        task_match = any(task in task_description for task in persona.sample_tasks)
        
        if capability_match or task_match:
            selected.append(persona)
    
    return selected[:MAX_AGENTS_PER_PHASE]
```

**Why this is better than embeddings:**
- Deterministic — same task → same agents every time
- Explainable — Queen can say "I chose Coder A because schema_design capability matched"
- Fast — string matching, not vector computation
- No dependencies — no OpenAI API, no vector DB

---

## 4. Books to Read

### Available Now (Can Access)

| Book | Publisher | Year | Relevance | Priority |
|------|-----------|------|-----------|----------|
| **Building Agentic AI Systems** | Packt / O'Reilly | 2024 | Multi-step planning, tool use, coordinator/worker/delegator pattern | 🔥 HIGH |
| **Anthropic's "Building Effective Agents"** | Blog (free) | 2025 | Prompt chaining, routing, evaluator-optimizer | 🔥 HIGH |
| **Google ADK Documentation** | adk.dev (free) | 2025 | Context engineering, tiered context, multi-agent | 🔥 HIGH |

### Need to Acquire (Recommended)

| Book | Author | Why Read It |
|------|--------|-------------|
| **Designing Data-Intensive Applications** | Martin Kleppmann | Database schema design, migrations, consistency — critical for our DB extensions |
| **An Introduction to MultiAgent Systems** | Michael Wooldridge | The academic foundation. Agent communication, coordination, game theory |
| **The Art of Agentic AI** | (Various, 2025 releases) | Likely covers latest patterns |

### Our Own Library (Already Ingested)

From the 8-book swarm knowledge base:
- **Agentic AI + Knowledge Graphs** — 694K words, 1,847 chunks
- **How to Read a Book (Adler)** — Syntopical reading methodology
- **Steal Like an Artist (Kleon)** — Creative synthesis patterns

**Recommendation:** Read the Packt book first. It's practical and covers exactly what we need: coordinator/worker/delegator, which maps to Queen/Agents/Coordinators.

---

## 5. Synthesis — What We Should Actually Build

### Context Reset (Between Phases)

**Don't:** Spawn new subagents (timeout issues, expensive)  
**Don't:** Reset to zero (loses critical design decisions)  
**Do:** Compile a "Phase Briefing" — structured YAML with:
- Previous phase artifact (summarized)
- Current task spec
- Relevant decisions from prior phases
- What NOT to worry about (other agents' work)

### Agent Context (What Each Agent Knows)

**Mandatory in every prompt:**
1. Goal (epic/story/task chain)
2. Circle of Competence (knows/doesn't know/will ask)
3. Role in phase (topology, peers, reports_to)
4. Expectations (output format, tests, docs, commit message)
5. Relevant History (activated episodic chunks, not full dump)

### Semantic Selection (Queen Knows Court)

**Don't:** Vector embeddings + similarity search  
**Don't:** Load all 81 persona×personality combos  
**Do:** Explicit capability declarations in YAML + symbolic keyword matching  
**Result:** Deterministic, explainable, fast, no API dependencies

---

## 6. Updated Open Questions

| # | Question | Research Says | Needs Your Decision |
|---|----------|---------------|---------------------|
| 1 | Auto-approval thresholds? | Google ADK uses gate functions — programmable | Define: compile success = auto-approve? |
| 2 | Parallel stories in epics? | Google ADK supports hierarchical, dependency-based | Hard deps block, soft deps warn? |
| 3 | Agent pool per phase? | Symbolic selection from capability YAML | 9 personas sufficient, or need 12? |
| 4 | MouminA bridge? | ADK supports tool integration | Priority: now or after swarm stable? |
| 5 | Token cost tracking? | Not well supported by CLIs | Skip for now, track time instead? |
| **6** | **Context reset mechanism?** | **Tiered briefing, not spawn** | **Approve this approach?** |
| **7** | **Agent context protocol?** | **5-section YAML header** | **Approve this format?** |
| **8** | **Semantic selection?** | **Capability YAML + keyword** | **Approve over vector search?** |

---

## 7. Recommended Next Step

**Update the design document** with these research findings:
1. Replace "spawn new subagent per phase" with "compile phase briefing"
2. Add Agent Context Protocol section
3. Add Queen's Court Selection algorithm
4. Remove vector DB references
5. Add book references to wiki

Then start **Phase A (Foundation)** with the corrected approach.

---

*Research sources: E-mem (ICML 2026), Google ADK Context Engineering, LangMem, Microsoft ISE Multi-Agent Systems, Anthropic Building Effective Agents, AdaptOrch benchmark.*
