# OpenClaw Swarm — Phase D UI/UX Critique Round
**Date:** 2026-05-10
**Critics:** Claude Design, Gemini Stitch, Kimi Code
**Synthesized by:** Ayan (Queen's Architect)

---

## Executive Summary

Three agents reviewed the Phase D UI/UX design document. All raised valid points. The critiques cluster into four areas: framework choice, missing UX states, information architecture, and production readiness. This document captures every critique, who said it, and the final decision.

---

## 1. Framework Debate

### Gemini Stitch
- **Critique:** Maintaining HTML/JS inside a Rust string literal breaks IDE syntax highlighting and formatting.
- **Solution:** Use `include_str!("dashboard.html")` to load HTML from a separate file. Identical compile-time embedding, better DX.
- **Additional:** Build a lightweight reactive state store in vanilla JS. Direct DOM manipulation with 4 parallel agents blasting SSE events will turn into spaghetti and race conditions.
- **Additional:** Add a "Filter" toggle in the UI to isolate commits, letters, or system logs. Agents are chatty — humans will drown without filtration.
- **Additional:** Add a hover state on Agent Cards to allow individual pausing/intervention when an agent hallucinates or loops.

### Claude Design
- **Critique:** The "no frameworks" constraint is actually two constraints glued together: (1) must compile into Rust binary with no runtime network deps, and (2) vanilla DOM APIs only. The first is real. The second is self-imposed.
- **Solution:** Vendor Preact+htm (~6KB combined) into the binary. Write JSX-like components with zero build step on the user side. Saves ~40% of code.
- **Alternative:** At minimum, be honest in the doc about WHY (philosophy, not technical necessity).

### Kimi Code
- **Critique:** Vanilla JS at this complexity becomes 2000+ lines of fragile DOM manipulation. The right approach: build with React + Vite, compile to production, then embed the output.
- **Counter:** Breaks the embedded binary constraint completely. 150KB of JS in a Rust binary is silly.

### Final Decision (Ayan)
**Go with Gemini's approach:** Vanilla JS + lightweight reactive state store + `include_str!` for HTML file. Why:
- Zero external dependencies = zero security audit, zero version drift
- If we hit 3000+ lines and it's painful, we'll vendor Preact+htm in v1.1
- React+Vite breaks the core constraint — non-starter
- Preact+htm adds 6KB and complexity we don't need for v1.0

---

## 2. Missing UX States

### Empty States
- **Who:** Kimi (strongest), Claude (implied)
- **What:** No designed empty state. First-time user sees blank gray screen. No "Create your first Epic" CTA. No demo data.
- **Severity:** 🔴 BUILD BLOCKING
- **Fix:** Every screen needs empty state: "No active epics — [Create First Epic]", "No phases started", "No activity yet" with large emoji + CTA button.

### Error/Failure States
- **Who:** Claude + Kimi (both flagged)
- **What:** SSE disconnects — what happens? API returns 500 — what shows? Agent crashes mid-phase — how's that visualized? No retry indicators, no "reconnecting" state design, no agent crash UI.
- **Severity:** 🔴 BUILD BLOCKING
- **Fix:** SSE disconnect banner (auto-reconnect countdown), API 500 retry with backoff, agent crash card with error stack + "Retry Agent" button.

### Activity Stream Noise
- **Who:** Claude + Gemini + Kimi (ALL three flagged this)
- **What:** Raw chronological dump. 4 agents firing 20 events in 10 seconds = unreadable noise. No grouping, no filtering, no collapsing rapid-fire events.
- **Severity:** 🔴 BUILD BLOCKING
- **Fix:** Filter bar: `All | Commits | Letters | Errors | System`. Group rapid-fire events: "Coder A completed 5 tasks [+2]". Default filter = All.

### Notification System
- **Who:** Claude + Kimi
- **What:** Phase needs approval → user must be staring at dashboard to notice. No toast alerts, no browser notifications, no badge counters. In a supervision tool, missing an approval is a blocker.
- **Severity:** 🔴 BUILD BLOCKING
- **Fix:** Toast stack (top-right) for "Phase needs approval" and "Agent errored". Badge on epic/story cards. Browser notification API for "Reviewing" state.

---

## 3. Information Architecture Issues

### Action Bar Danger
- **Who:** Claude
- **What:** Bottom action bar puts `[Approve Phase]` and `[Abort]` in same horizontal strip. Approve is most common; Abort is destructive and irreversible.
- **Severity:** 🔴 BUILD BLOCKING
- **Fix:** `[Approve Phase]` (green, left, primary) · `[Request Changes]` (yellow) · `[Pause]` (blue) · `···` overflow → `[Abort]` (red, requires confirmation typing story name or 3-second hold).

### Dependencies Hidden
- **Who:** Claude
- **What:** Story DAG lives in data model but is behind a "view dependency graph ↗" link as a modal. Dependencies should be ambient — inline on Epic Board (visual blockers on cards) and on Story Phase View ("blocked by Story 1" pill on header).
- **Severity:** 🔴 BUILD BLOCKING
- **Fix:** Story cards show blocker pill: "⏸️ Blocked by Story 1". Phase header shows "Waiting for Story X".

### Phase Progress Bar Wrong
- **Who:** Claude
- **What:** Progress = `approved_phases / total_phases` — but that's story progress shown on the phase card. Phase itself needs internal progress: X of Y agents complete, or X of Y artifacts written.
- **Severity:** 🔴 BUILD BLOCKING
- **Fix:** Progress bar uses `completed_agents / total_agents` while phase runs. Shows actual work happening.

### Activity Grouping
- **Who:** Kimi
- **What:** Unstructured chronological dump. When 3 agents fire 20 events in 10 seconds, noise.
- **Severity:** 🔴 BUILD BLOCKING
- **Fix:** Collapsible agent event groups. "Coder A completed 5 tasks [expand]".

### No Global Overview
- **Who:** Kimi
- **What:** Doc jumps straight to Epic Board. No "home" view. Human supervisor needs command center: pending approvals across ALL stories, agent health summary, today's velocity, recent failures.
- **Severity:** 🟡 SIGNIFICANT
- **Fix:** New "Overview Dashboard" (Screen 0): Pending approvals count, active agents, today's stories completed, recent errors, velocity sparkline.

### Agent Health Missing
- **Who:** Kimi
- **What:** Agents error, retry, timeout, hang. Status dots (🟢🟡🔴) only. No error history, no retry count, no "last 10 runs" sparkline.
- **Severity:** 🟡 SIGNIFICANT
- **Fix:** Agent card expanded: error history (last 5), retry count, uptime sparkline (last 10 min), token cost for this phase.

### Token/Cost Invisible
- **Who:** Claude
- **What:** This is an LLM agent orchestrator. Token cost, model selection, rate-limit status — nowhere. Burn $400 on a runaway debate loop with no visibility.
- **Severity:** 🟡 SIGNIFICANT
- **Fix:** Phase header shows `💰 12.4K tokens · $0.43`. Per-phase cost from `phase_metrics` table.

### URL Routing Missing
- **Who:** Claude
- **What:** Three screens, deep links, browser back button, shareable links to "this phase right now" — none specified.
- **Severity:** 🟡 SIGNIFICANT
- **Fix:** Hash-based routing: `#/epic/{id}`, `#/story/{id}`, `#/story/{id}/phase/{id}`. Browser back button works.

### Keyboard Shortcuts Under-specified
- **Who:** Kimi
- **What:** j/k navigation is good, but missing: approve (a), reject (r), switch epic (Cmd+1-9), focus search (/), toggle sidebar ([). Linear's shortcut game is leagues ahead.
- **Severity:** 🟡 SIGNIFICANT
- **Fix:** `a` = approve, `r` = request changes, `p` = pause, `1-9` = switch epic, `/` = search, `?` = shortcut help overlay, `[` = toggle sidebar.

### Who Approved — Invisible
- **Who:** Kimi
- **What:** `approved_by` exists in data model but isn't shown in UI. When phase is approved, who approved it? Critical for audit trails.
- **Severity:** 🟡 SIGNIFICANT
- **Fix:** Phase detail shows "Approved by Shadab · 2026-05-10 14:32". Review report shows reviewer name.

---

## 4. Production Readiness Gaps

### Mobile Afterthought
- **Who:** All three mentioned
- **What:** "Stacked layout with swipeable carousel" — barely specified. No bottom nav, no pull-to-refresh, no touch-optimized action bar.
- **Severity:** 🟢 NICE-TO-HAVE
- **Decision:** 27" monitor is primary target. Mobile = v1.1.

### Accessibility/WCAG
- **Who:** Kimi
- **What:** `--agent-planner: #2f81f7` on `--bg-primary: #0d1117` is borderline contrast. No ARIA labels, no focus management, no reduced motion.
- **Severity:** 🟢 NICE-TO-HAVE
- **Decision:** Dev tool for technical users. Skip v1.0, improve v1.1.

### Export/Share
- **Who:** Kimi
- **What:** Can't share a phase result, export artifacts as zip, copy a link to a specific story.
- **Severity:** 🟢 NICE-TO-HAVE
- **Decision:** Collaboration feature. Skip v1.0.

### Branch Status Buried
- **Who:** Claude
- **What:** Git branch names are in data model but barely surfaced. For a dev tool, branch status (ahead/behind, open PR) should be prominent.
- **Severity:** 🟢 NICE-TO-HAVE
- **Decision:** Skip v1.0.

### Ship Phase Undesigned
- **Who:** Claude
- **What:** Despite being Vercel-inspired finale, Ship phase has zero design.
- **Severity:** 🟢 NICE-TO-HAVE
- **Decision:** Skip v1.0 — currently just a status badge.

---

## 5. Critiques I Disagree With

| Critique | Who | Why I Push Back |
|----------|-----|-----------------|
| Left column is redundant | Kimi | NOT a duplicate timeline — left is **navigation** (click to jump between phases), center is **detail** (activity for selected phase). Different jobs. |
| Use React + Vite + Tailwind | Kimi | Breaks the core constraint. 150KB of JS in a Rust binary is silly. Preact+htm if we MUST, but Gemini's vanilla+store is better. |
| Prototype sprint first | Claude | We already HAVE a working v0.2 dashboard. We know what works. Build the real thing. |
| Full attribution is missing | Kimi | Already in data model (`approved_by`, `timestamp`). Surface it — 2 lines of UI, not a missing feature. |

---

## 6. Final Priority Matrix

### 🔴 MUST FIX (v1.0 — 8 items)
1. Empty states
2. Error/failure states
3. Activity stream filters
4. Notification system
5. Safe action bar (Approve/Abort separation)
6. Inline dependencies (blocker pills)
7. Phase internal progress (agent-based, not phase-based)
8. Activity grouping

### 🟡 SHOULD FIX (v1.0 — 7 items)
9. Global overview dashboard (Screen 0)
10. Agent health cards
11. Token/cost visibility
12. URL hash routing
13. Keyboard shortcuts expansion
14. Who approved attribution
15. include_str! for HTML file

### 🟢 NICE-TO-HAVE (v1.1 — 6 items)
16. Mobile responsive
17. Dark/light mode
18. Accessibility/WCAG
19. Export/share
20. Branch status actionable
21. Ship phase design

---

## 7. What Each Critic Got Right

### Gemini Stitch
- `include_str!` is objectively better than inline string
- Reactive state store in vanilla JS is the right middle ground
- Activity filters are essential for agent noise
- Individual agent pause/kill is a real need

### Claude Design
- Preact+htm is technically a valid option (just not the one we picked)
- Failure UX is half of what an orchestrator's life looks like
- Action bar danger is a real safety issue
- Dependencies should be ambient, not hidden
- Token cost visibility will prevent $400 surprises
- URL routing is a basic SPA requirement

### Kimi Code
- Empty states are product killers
- Error states are where users actually spend time
- Global overview dashboard is essential for supervisors
- Agent health without metrics is flying blind
- Keyboard shortcuts should be comprehensive
- Attribution is critical for audit trails

---

## 8. What I Learned

1. **Three critics > one.** Each caught things the others missed. Gemini caught reactive state. Claude caught failure modes. Kimi caught supervision needs.
2. **The framework debate is really about team size.** Solo builder = vanilla is fine. Team of 3+ = Preact saves time. We're solo for now.
3. **Failure states are 50% of the UX.** We designed the happy path beautifully and ignored where users actually spend time. Never again.
4. **Agents produce noise at scale.** 4 agents × 5 events/min = 1200 events/hour. Without filters and grouping, the dashboard is unusable.
5. **Supervision is the job.** The user isn't "using" the dashboard — they're supervising agents. Overview dashboard, notifications, health metrics — these are the job.

---

*Synthesized by Ayan. Ready for build.*
