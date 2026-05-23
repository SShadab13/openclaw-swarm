# Swarm + TUI Interaction Guide

**For:** Shadab (swarm owner)
**Date:** May 9, 2026
**Version:** openclaw-swarm v0.1.0

---

## Quick Start

```bash
# 1. Navigate to swarm directory
cd C:\Users\shada\.kimi_openclaw\workspace\openclaw-swarm

# 2. Run the TUI dashboard
.\target\release\openclaw-swarm.exe dashboard

# 3. Or use cargo
cargo run --release --bin openclaw-swarm -- dashboard
```

**Dashboard Controls:**
- `q` or `Esc` — Quit
- `↑/↓` or `j/k` — Navigate tasks list
- `←/→` or `h/l` — Navigate agents table
- `PgUp/PgDn` — Scroll mail panel
- `r` — Force refresh data
- Auto-refresh every 5 seconds

---

## Swarm Commands

### Initialize (first time only)
```bash
openclaw-swarm init
# Creates: openclaw-swarm.db, personas/, personalities/, workspace/
```

### Create a Task
```bash
openclaw-swarm task \
  --name "build-auth-system" \
  --description "OAuth2 + JWT authentication for API" \
  --task-type sdlc_feature
# Output: Task ID + branch name + agent assignments
```

### Start a Task (swarm begins working)
```bash
openclaw-swarm start --task-id <ID>
# Swarm agents begin execution
```

### Check Status
```bash
openclaw-swarm status --task-id <ID>
# Shows: swarm mood, step status, action required
```

### Run Execution Loop (until completion)
```bash
openclaw-swarm run --task-id <ID>
# Runs the execution loop: dispatch agents, collect output, track progress
```

### List Active Tasks
```bash
openclaw-swarm list
```

### Ship (merge to main)
```bash
openclaw-swarm ship --task-id <ID>
# Only works when all agents report DONE
```

### Send Letter (inter-agent communication)
```bash
openclaw-swarm letter \
  --task-id <ID> \
  --from architect \
  --to coder \
  --content "Please refactor the auth middleware" \
  --mood focused
```

### Write Diary (private reflection)
```bash
openclaw-swarm diary \
  --task-id <ID> \
  --persona coder \
  --personality tsundere \
  --entry "The auth logic is messier than I thought. Need to simplify." \
  --mood frustrated
```

### Reassign Personality (Queen's command)
```bash
openclaw-swarm reassign \
  --task-id <ID> \
  --persona tester \
  --personality sadist_cheerful \
  --mood excited \
  --reason "Need aggressive edge case testing"
```

---

## Dashboard Tabs

### Tab 1: Tasks
- Shows all active tasks from `openclaw-swarm.db`
- Columns: Status | Name | Agents | Branch
- Select a task to see its details in the right panel

### Tab 2: Agents
- Shows all agent assignments from `task_agents` table
- Columns: Task | Persona | Personality | Mood
- Navigate with arrow keys

### Tab 3: Knowledge
- Shows stats from `swarm_knowledge.db`
- Concepts: ~809 (from 8 books)
- Books: 8 ingested
- Chunks: ~1,800
- Links: Syntopical connections

### Tab 4: Runners
- Shows runner status: Kimi, Claude, OpenClaw
- Online/offline indicator
- Version info

---

## Database Files

| Database | Path | Content |
|----------|------|---------|
| Tasks | `openclaw-swarm.db` | Tasks, agents, letters, diary |
| Knowledge | `scripts/swarm_knowledge.db` | Concepts, books, chunks, links |
| Error Journal | `error_journal.db` | Failure patterns, fixes |

---

## Workflow Example

```bash
# 1. Create task
$ openclaw-swarm task --name "tui-dashboard" --description "Build ratatui dashboard" --task-type sdlc_feature
Task 0c192311-1bef-4175-9e22-a850332b34ef created with sdlc_feature swarm
Branch: swarm/tui-dashboard/47d92cd5
Run: openclaw-swarm start --task-id 0c192311-1bef-4175-9e22-a850332b34ef

# 2. Start task
$ openclaw-swarm start --task-id 0c192311-1bef-4175-9e22-a850332b34ef
Task started. Swarm is active.

# 3. Open dashboard (in another terminal)
$ openclaw-swarm dashboard

# 4. Run execution loop
$ openclaw-swarm run --task-id 0c192311-1bef-4175-9e22-a850332b34ef

# 5. Check status
$ openclaw-swarm status --task-id 0c192311-1bef-4175-9e22-a850332b34ef

# 6. Ship when done
$ openclaw-swarm ship --task-id 0c192311-1bef-4175-9e22-a850332b34ef
```

---

## Troubleshooting

### Dashboard shows no data
- Check DB files exist: `openclaw-swarm.db`, `scripts/swarm_knowledge.db`
- Run `openclaw-swarm init` if missing

### Swarm agents timeout
- Current limitation: `sessions_spawn` has ~2.5m cap
- Workaround: Use smaller tasks or run manually
- Fix in progress: Persistent execution workers

### Build errors
- Ensure cargo is available: `C:\Users\shada\.cargo\bin\cargo.exe`
- Run `cargo check` for diagnostics

---

## Advanced: Querying the Knowledge Base

```bash
# Python query example
python -c "
import sqlite3
conn = sqlite3.connect('scripts/swarm_knowledge.db')
c = conn.cursor()
c.execute('SELECT name, definition FROM concepts LIMIT 5')
for row in c.fetchall():
    print(f'{row[0]}: {row[1][:100]}...')
"
```

---

*Generated: May 9, 2026*
*openclaw-swarm v0.1.0*
