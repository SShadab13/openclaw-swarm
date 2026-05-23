# OpenClaw Swarm — Phase D: UI/UX Design Document
**Status:** Design Complete — Ready for Build (v1.2 — Post-Kimi Gift)
**Date:** 2026-05-10
**Author:** Ayan (Queen's Architect)
**Contributors:** Claude Design, Gemini Stitch, Kimi Code (critique + build round)
**Visual Reference:** Kimi's React Build (kimi-ui-gift/app/)
**Research Sources:** Linear.app, GitHub Actions, Railway.app, Warp Terminal

---

## 🎯 For AI Agent Builders (Claude Design + Gemini Stitch)

**You are building the frontend for a multi-agent orchestration dashboard.** The backend is complete (Phases A+B+C). You are writing the HTML/CSS/JS that gets embedded into a Rust binary.

**Visual Reference:** Kimi Code built a complete React + Tailwind + shadcn/ui dashboard as a "gift" while critiquing. We are NOT using it directly (wrong tech stack), but we are copying its design patterns pixel-for-pixel. Open `kimi-ui-gift/app/src/pages/` to see the target designs.

### Tech Stack — FINAL (v1.2)

| Layer | Technology | Why |
|-------|-----------|-----|
| HTML | Single file, `include_str!("dashboard.html")` | Separate file for syntax highlighting, embedded at compile time |
| CSS | Inline `<style>` block | No external files, no CDN |
| JavaScript | Vanilla JS + lightweight reactive store | No React. But no `querySelector` spaghetti either. Kimi's React build = visual reference only. |
| Icons | Emoji + Unicode | `🤖` `✅` `▶️` `⏸️` `🔴` `⚠️` `💰` `🔥` |
| HTTP | Fetch API | Standard browser fetch |
| Real-Time | SSE (EventSource) + auto-reconnect | Reconnect with exponential backoff |

**Framework decision — FINAL:** Vanilla JS + reactive store. Kimi's React build proved the design works. Now we rebuild it in vanilla JS to meet the "embedded binary" constraint. If we hit 3000+ lines, we vendor Preact+htm in v1.1.

**You write ONE self-contained HTML document:** `src/dashboard.html`

---

## 🎨 What We Steal from Kimi's React Build

### Color Palette (UPDATED — Kimi's is better)

```css
:root {
  /* Backgrounds */
  --bg-primary: #0A0A0A;        /* Deepest black — Kimi's choice */
  --bg-secondary: #111111;        /* Card background */
  --bg-tertiary: #1a1a1a;        /* Hover states */
  --bg-elevated: #222222;         /* Elevated cards */
  
  /* Borders */
  --border-subtle: rgba(255,255,255,0.05);
  --border-medium: rgba(255,255,255,0.10);
  --border-strong: rgba(255,255,255,0.15);
  
  /* Text */
  --text-primary: #ffffff;
  --text-secondary: rgba(255,255,255,0.60);
  --text-muted: rgba(255,255,255,0.40);
  --text-disabled: rgba(255,255,255,0.25);
  
  /* Accents — Kimi's gold-primary palette */
  --accent-primary: #F6A94C;      /* Gold — primary CTAs, active states */
  --accent-primary-dim: rgba(246,169,76,0.15);
  
  /* Status */
  --status-pending: #8b949e;
  --status-running: #58a6ff;      /* Blue */
  --status-blocked: #f59e0b;      /* Amber */
  --status-reviewing: #a855f7;    /* Purple */
  --status-approved: #10b981;     /* Emerald */
  --status-rejected: #ef4444;     /* Red */
  --status-skipped: #6e7681;
  --status-error: #ef4444;
  
  /* Agent Colors */
  --agent-architect: #58a6ff;
  --agent-coder: #10b981;
  --agent-tester: #ef4444;
  --agent-frontend: #f59e0b;
  --agent-devops: #a855f7;
  --agent-queen: #f472b6;
  --agent-coordinator: #d8b4fe;
  --agent-planner: #3b82f6;
  --agent-security: #f87171;
  --agent-performance: #fb923c;
  
  /* Toast/Notification */
  --toast-info: #58a6ff;
  --toast-success: #10b981;
  --toast-warning: #f59e0b;
  --toast-error: #ef4444;
}
```

### Component Patterns (from Kimi)

| Component | Kimi's Pattern | Our Vanilla JS Adaptation |
|-----------|---------------|---------------------------|
| **Hero Section** | Gradient background + grid pattern overlay + personalized greeting + stats summary | Same layout, CSS `linear-gradient` + SVG grid pattern |
| **Metric Card** | Icon + label + value + colored background tint | Same, with emoji icon + CSS `background: color/10%` |
| **Epic Card** | Expandable card with progress bar + story grid | Same, vanilla JS `classList.toggle('expanded')` |
| **Story Card** | Status badge + name + duration + blocker pill | Same, CSS flex layout |
| **Phase Stepper** | Horizontal timeline with icons + connecting lines | Same, CSS flex + `::before` pseudo-elements for lines |
| **Activity Item** | Timestamp + actor icon + action text | Same, with emoji actors |
| **Agent Card** | Avatar + name + status dot + sub-task + hover pause | Same, CSS hover state |
| **Toast** | Colored left border + title + message + actions | Same, CSS `border-left` + auto-dismiss |
| **Command Palette** | Modal overlay + search input + fuzzy results + keyboard nav | Same, vanilla JS filter + `keydown` nav |
| **Empty State** | Large centered icon + title + description + CTA | Same, with emoji icon |

### Layout Structure (from Kimi)

```
┌─────────────────────────────────────┐
│  Sidebar (60px) │  Main Content       │
│  ┌─────────┐   │  ┌───────────────┐   │
│  │ 🏠      │   │  │  TopBar       │   │
│  │ 📋      │   │  ├───────────────┤  │
│  │ ⚡      │   │  │               │   │
│  │ 🤖      │   │  │  Page Content │   │
│  │ 🏆      │   │  │               │   │
│  │ ⚙️      │   │  └───────────────┘   │
│  └─────────┘   │                      │
└─────────────────────────────────────┘
```

**Sidebar:** Icon-only navigation (60px wide), vertical stack, active state with left border accent.
**TopBar:** Breadcrumbs + page title + action buttons + user avatar.
**Main Content:** Scrollable, padding 24px.

---

## 1. Design Philosophy

### What We Steal (and From Where)

| From | What We Steal | Applied To |
|------|--------------|------------|
| **Linear.app** | Clean issue lists, status pills, Cmd+K search | Epic cards, activity stream |
| **GitHub Actions** | Step pipeline visualization | Phase timeline |
| **Kimi's React Build** | Color palette, layout structure, component patterns | Everything — our visual reference |
| **Warp Terminal** | Terminal as agent surface | Dashboard vibe |
| **Vercel Dashboard** | Deployment progress, real-time logs | Ship phase |

### Core Principles

1. **Progressive Disclosure** — Default view shows active story + current phase. Everything else is a click away.
2. **Agent-First** — Built for agents to produce, humans to supervise.
3. **Action at Every Level** — Every screen has a primary action.
4. **Real-Time by Default** — SSE pushes updates. No refresh needed.
5. **Color = Information** — Status colors carry semantic meaning.
6. **Keyboard-First** — Cmd+K search, `j/k` nav, `Enter` to act, `Esc` to back out.

---

## 2. Information Architecture

### Four-Level Hierarchy

```
Level 0: Overview Dashboard (Command Center)
├── "What's happening right now?"
├── Pending approvals, active agents, velocity, errors
└── Entry point every time you open the app

Level 1: Epic Board (Strategic View)
├── "What are we building?"
├── All epics, story cards, dependency graph
└── Entry point for new work

Level 2: Story Phase View (Tactical View)
├── "Where are we in the process?"
├── Phase timeline, active agents, activity stream
└── Where humans spend 80% of their time

Level 3: Detail Drill-Down (Operational View)
├── "What exactly happened?"
├── Agent conversations, file diffs, review findings
└── Deep-dive for debugging or review
```

---

## 3. Screen Specifications

### Screen 0: Overview Dashboard (Command Center)

**Purpose:** One glance tells you everything that needs attention.

**Layout (from Kimi):**
```
┌──────────────────────────────────────────────────────────────────┐
│  🔷 OpenClaw Swarm                                    [🔔 3]    │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 🔥 Active Streak: 5 Days                                  │  │
│  │                                                          │  │
│  │ Welcome back, Architect                                  │  │
│  │ You have 4 pending approvals, 12 active agents,           │  │
│  │ and 3 phases running.                                    │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │ 🔔 4     │ │ 🤖 12    │ │ ⚡ 3     │ │ 📈 1.4x  │          │
│  │ Pending  │ │ Active   │ │ Running  │ │ Velocity │          │
│  │ Approvals│ │ Agents   │ │ Phases   │ │ Today    │          │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘          │
│                                                                  │
│  ⚠️ Needs Your Attention (3)                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ ▶️ Phase "Implementation" needs your approval            │  │
│  │    Story: Activity Feed · Epic: Social Tab Rebuild      │  │
│  │    [Review Now]                                          │  │
│  ├──────────────────────────────────────────────────────────┤  │
│  │ 🔴 Agent Coder A errored in "Friendship System"          │  │
│  │    Error: Timeout after 30s · Retry count: 2               │  │
│  │    [View Error] [Retry Agent]                              │  │
│  ├──────────────────────────────────────────────────────────┤  │
│  │ ⏸️ Story "Messaging" blocked by "Friendship System"       │  │
│  │    Dependency: hard · Waiting for completion             │  │
│  │    [View Blocker]                                        │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  📈 Velocity (last 24h)                                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ █ █ █ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░ ░   │  │
│  │ 12 stories · 45 commits · 3 errors · 2 replans         │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  🏗️ Active Epics (2)                                             │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Social Tab Rebuild    ████████░░ 40% · 2/5 stories     │  │
│  │ Profile Redesign      ██████░░░░ 25% · 1/4 stories       │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

**Data Sources:**
- `GET /api/tasks` — all tasks
- `GET /api/activity/{story_id}` — recent errors
- `GET /api/agents/{phase_id}` — active agent count

---

### Screen 1: Epic Board

**Purpose:** Strategic overview. See all work, start new epics.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────┐
│  🔷 Epic Board                              [+ New Epic] [🔍]   │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Filter: [All ▼]  Search: [___________🔍]                      │
│                                                                  │
│  Active Epics (2)                                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 🏗️ Social Tab Rebuild              [⏱️ 2h 15m]          │  │
│  │ ████████████░░░░░░ 40% (2/5 stories)                     │  │
│  │                                                          │  │
│  │ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐      │  │
│  │ │ ✅ Story 1   │ │ ▶️ Story 2   │ │ ⏸️ Story 3   │      │  │
│  │ │ Friendship   │ │ Activity Feed│ │ Messaging    │      │  │
│  │ │ [12m]        │ │ [45m]        │ │ ⏸️ Blocked   │      │  │
│  │ │ [view]       │ │ [view live]  │ │ by Story 1   │      │  │
│  │ └──────────────┘ └──────────────┘ └──────────────┘      │  │
│  │                                                          │  │
│  │ Dependencies: Story 4 → Story 1                          │  │
│  │ [view dependency graph ↗]                                │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Completed Epics (3)  [expand ▼]                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ ✅ Day Seal              SHIPPED · 5 phases · 0 issues   │  │
│  │ [view history]                                           │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                                                          │  │
│  │                    🏗️                                   │  │
│  │                                                          │  │
│  │              No active epics yet                         │  │
│  │                                                          │  │
│  │       Create your first epic to start building            │  │
│  │       with your agent swarm                              │  │
│  │                                                          │  │
│  │            [+ Create First Epic]                         │  │
│  │                                                          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

### Screen 2: Story Phase View — PRIMARY

**Purpose:** Where humans spend 80% of time.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────┐
│  ← Social Tab Rebuild / Activity Feed              [⏱️ 45m]    │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Phase Timeline                                                  │
│  ●━━━━━●━━━━━▶━━━━━○━━━━━○                                      │
│  ✅    ✅    ▶️     ⏸️     ⏸️                                      │
│  Plan  Design Impl  Review Ship                                  │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Phase 3: Implementation                                  │  │
│  │ ████████░░░░ 80% · Topology: parallel · 3 of 4 agents   │  │
│  │                                                          │  │
│  │ 💰 12.4K tokens · $0.43 · ⏸️ Waiting for Story 1          │  │
│  │                                                          │  │
│  │ [▶ Start] [⏸ Pause]                                     │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Activity Stream          [All ▼] [🤖 Coder A ▼]             │
│  ─────────────────────────────────────────────────────────     │
│                                                                  │
│  [07:52:15] 🤖 Coder A → "Created migration"                │
│             v5_add_day_seal.sql                                  │
│                                                                  │
│  [07:52:30] 🤖 Coder B → "Wrote calculateDaySeal()"          │
│             services/xp.ts                                       │
│                                                                  │
│  [07:52:45] ⚙️ Auto-check → "✅ Compile OK"                  │
│                                                                  │
│  [07:53:00] 💬 Letter: Coder A → Coder D                      │
│             "Need user.id vs userId — which does UserProvider   │
│              use?"                                               │
│                                                                  │
│  [07:53:15] 🤖 Coder D → "user.id — confirmed"               │
│                                                                  │
│  [07:54:00] 🤖 Coder A → "Completed 3 tasks [+2 ▼]"          │
│                                                                  │
│  ─────────────────────────────────────────────────────────     │
│                                                                  │
│  Active Agents (3)                    Pending (1)              │
│  ┌────────┐ ┌────────┐ ┌────────┐   ┌────────┐                 │
│  │ 🤖     │ │ 🤖     │ │ 🤖     │   │ 🤖     │                 │
│  │ Coder A│ │ Coder B│ │ Coder C│   │ Coder D│                 │
│  │ 🟢 Live│ │ 🟡 Busy│ │ 🟡 Busy│   │ ⏸️ Wait│                 │
│  │db done │ │xp svc  │ │UI badge│   │wiring  │                 │
│  └────────┘ └────────┘ └────────┘   └────────┘                 │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│  [✅ Approve]  [📝 Request Changes]  [⏸ Pause]  [···]          │
│                                        └─► [⚠️ Abort]         │
└──────────────────────────────────────────────────────────────────┘
```

---

### Screen 3: Phase Detail

**Purpose:** Deep dive into artifacts, letters, commits, review.

**Layout:**
```
┌──────────────────────────────────────────────────────────────────┐
│  ← Back                                          Phase 3: Impl  │
├──────────────────────────────────────────────────────────────────┤
│  Status: ✅ Approved by Shadab · 2026-05-10 14:32 · 7m duration │
│  💰 12.4K tokens · $0.43 · 4 agents · 0 issues                  │
├──────────────────────────────────────────────────────────────────┤
│  [Artifacts] [Letters] [Commits] [Diffs] [Review]               │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  📁 Artifacts (4)                                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 📄 plan.md              Planning doc · 2KB               │  │
│  │ 📄 design.md            Architecture · 5KB               │  │
│  │ 📄 migration.sql        DB schema · 1KB                │  │
│  │ 📄 services/xp.ts       Implementation · 8KB          │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  💬 Letters (6)                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ 🤖 Coder A → Coder D                                    │  │
│  │ "Need to clarify user.id vs userId"                     │  │
│  │                                                    07:52 │  │
│  ├──────────────────────────────────────────────────────────┤  │
│  │ 🤖 Coder D → Coder A                                    │  │
│  │ "user.id — confirmed. Check db/client.ts L42."         │  │
│  │                                                    07:53 │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  🔍 Review Report                                                │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ Overall: Approve with Notes                              │  │
│  │ Reviewer: Security Reviewer · 2026-05-10 14:30           │  │
│  │                                                          │  │
│  │ ⚠️ Simplicity (1 warning)                                │  │
│  │ "Consider extracting magic number to constant"            │  │
│  │                                                          │  │
│  │ ✅ Bugs (0 issues)                                       │  │
│  │ ✅ Conventions (0 issues)                                │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 4. Real-Time Architecture

### SSE with Auto-Reconnect

```javascript
let evtSource;
let reconnectDelay = 3000;
let maxReconnectDelay = 30000;

function connectSSE() {
  evtSource = new EventSource('/events');
  
  evtSource.onopen = () => {
    hideDisconnectBanner();
    reconnectDelay = 3000; // Reset on success
  };
  
  evtSource.onmessage = (event) => {
    const data = JSON.parse(event.data);
    handleSSEEvent(data);
  };
  
  evtSource.onerror = () => {
    evtSource.close();
    showDisconnectBanner(`Reconnecting in ${reconnectDelay/1000}s...`);
    setTimeout(connectSSE, reconnectDelay);
    reconnectDelay = Math.min(reconnectDelay * 2, maxReconnectDelay);
  };
}

function handleSSEEvent(data) {
  switch(data.type) {
    case 'activity':
      prependActivity(data);
      break;
    case 'phase_status':
      updatePhaseStatus(data.phase_id, data.new_status);
      break;
    case 'agent_status':
      updateAgentCard(data.persona_id, data.status);
      break;
    case 'phase_needs_approval':
      showToast('Phase needs approval', data.phase_name, 'warning');
      updateBadgeCounter('+1');
      break;
    case 'agent_error':
      showToast('Agent error', `${data.persona_id}: ${data.error}`, 'error');
      updateAgentCard(data.persona_id, 'error');
      break;
  }
}
```

---

## 5. Keyboard Shortcuts

| Key | Action | Context |
|-----|--------|---------|
| `j` | Navigate down | Activity stream, lists |
| `k` | Navigate up | Activity stream, lists |
| `Enter` | Select/expand | Anywhere |
| `Esc` | Close modal/panel | Anywhere |
| `a` | Approve phase | Story Phase view |
| `r` | Request changes | Story Phase view |
| `p` | Pause/resume | Story Phase view |
| `1-9` | Switch epic | Epic Board |
| `/` or `Cmd+K` | Open search | Anywhere |
| `?` | Show shortcuts help | Anywhere |
| `[` | Toggle sidebar | Anywhere |
| `n` | New epic | Epic Board |

---

## 6. Responsive Behavior

### Desktop (> 1024px) — Three Columns
Story Phase: Left 25% | Center 50% | Right 25%

### Tablet (768px - 1024px) — Two Columns
Left 40% (tree) | Center 60% (phases)

### Mobile (< 768px) — Stacked (v1.1)
Not a priority. Primary user is on 27" monitor.

---

## 7. Implementation Plan

### Phase D.1: API Completion (30 min)
1. Add 8 missing API endpoints to `web_dashboard.rs`
2. Add `health_score`, `error_count`, `retry_count` to models
3. `cargo check`

### Phase D.2: Vanilla JS Dashboard (90 min)
1. Create `src/dashboard.html`
2. Copy Kimi's color palette, layouts, component patterns
3. Build 4 screens: Overview, Epic Board, Story Phase, Phase Detail
4. Add reactive state store
5. Include mock mode

### Phase D.3: Polish (30 min)
1. Keyboard shortcuts
2. URL hash routing
3. SSE auto-reconnect
4. Toast notifications
5. `cargo check` + test

**Total: ~2.5 hours**

---

## 8. Acceptance Criteria

- [ ] Overview Dashboard with hero + metrics + attention list
- [ ] Epic Board with search + filter + expandable cards
- [ ] Story Phase with phase timeline + activity + agents
- [ ] Activity stream with filters + grouping
- [ ] Approve/Reject/Request Changes buttons work
- [ ] Safe action bar (Abort behind overflow)
- [ ] Click artifact → show file content
- [ ] Click letter → show conversation
- [ ] Dependency blockers inline on cards
- [ ] Empty states for all screens
- [ ] Error states: SSE disconnect, API retry, agent crash
- [ ] Toast notifications for approvals + errors
- [ ] Keyboard shortcuts (j/k, Enter, Esc, a, r, p, /, ?)
- [ ] URL hash routing
- [ ] Token cost visible in phase header
- [ ] "Who approved" attribution
- [ ] Agent health score + error count
- [ ] No console errors
- [ ] `cargo check` passes

---

## 10. Migration Path — When Requirements Change

**Recorded:** 2026-05-10 — Shadab approved vanilla JS for v1.0, with migration plan ready.

### Trigger Conditions

| Trigger | Current State | Migration Target | Effort |
|---------|-------------|------------------|--------|
| Vanilla JS hits 3000+ lines and is unmaintainable | ~1500 lines estimated for v1.0 | Preact + htm (~6KB) | 1 day — swap reactive store for Preact state, keep all components |
| Need complex animations | CSS transitions only | Framer Motion (vendored) | 2 days — add animation layer |
| Need mobile app | Responsive CSS (v1.1) | React Native (separate repo) | 1 week — new project |
| Need SaaS with user accounts | Single user (you) | Full React + auth + billing | 1 month — dedicated frontend team |
| Need desktop app with native menus | Browser dashboard | Tauri | 3 days — wrap binary in Tauri |
| Need GPU-accelerated viz | CSS charts only | egui | 1 week — rewrite dashboard in egui |

### Preact + htm Migration Plan (Most Likely)

**Why Preact + htm?**
- 6KB total (vs React's 40KB)
- Same API as React (components, hooks, JSX-like via htm)
- Zero build step — htm is a template literal function
- Can be vendored into the binary (single JS file)
- `include_str!("preact.min.js")` + `include_str!("htm.min.js")` + `include_str!("dashboard.js")`

**Migration Steps:**
1. Vendor `preact.min.js` + `htm.min.js` into `src/` (~6KB combined)
2. Convert reactive store components to Preact components
3. Use `html` template literal from htm instead of JSX
4. Keep all CSS, all API endpoints, all data shapes
5. `cargo build` — still one step

**Example:**
```javascript
// Current (vanilla JS)
function renderAgentCard(agent) {
    const div = document.createElement('div');
    div.className = 'agent-card';
    div.innerHTML = `🤖 ${agent.name}`;
    return div;
}

// Preact + htm (future)
import { html } from './htm.js';

function AgentCard({ agent }) {
    return html`
        <div class="agent-card">
            🤖 ${agent.name}
        </div>
    `;
}
```

### Why Not React?

If we migrate, we'd use **Preact + htm**, not React. Why:
- Preact is 3KB vs React's 40KB
- htm is 700 bytes vs Babel/webpack infrastructure
- Same component model, same hooks, same everything
- No build step — template literals compile at runtime

React only makes sense if we have:
- A dedicated frontend team
- A build pipeline (CI/CD with Node.js)
- A SaaS product with user accounts
- Complex state management (Redux, Zustand)

**None of these apply now.**

### Migration Decision Log

| Date | Decision | Trigger | Approved By |
|------|----------|---------|-------------|
| 2026-05-10 | Vanilla JS for v1.0 | Initial build | Shadab |
| Future | Preact + htm for v1.1 | Codebase hits 3000+ lines | TBD |
| Future | Full React for v2.0 | SaaS product, dedicated frontend team | TBD |

---

## 11. File Locations

| What | Where |
|------|-------|
| HTML Dashboard | `src/dashboard.html` |
| Rust loader | `src/web_dashboard.rs` — `include_str!("dashboard.html")` |
| API Handlers | `src/web_dashboard.rs` |
| New DB methods | `src/db.rs` |

---

**Visual Reference:** `kimi-ui-gift/app/src/pages/`
**Ready to build.**
