import sqlite3
import sys
import os

print('CWD:', os.getcwd())
print('DB exists:', os.path.exists('openclaw-swarm.db'))
if not os.path.exists('openclaw-swarm.db'):
    print('DB not found')
    sys.exit(1)

conn = sqlite3.connect('openclaw-swarm.db')
c = conn.cursor()

c.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='bridge_queue'")
tables = c.fetchall()
print('Tables:', tables)
if not tables:
    print('bridge_queue table not found')
    sys.exit(1)

c.execute("SELECT COUNT(*) FROM bridge_queue WHERE status='pending'")
count = c.fetchone()[0]
print('Pending count:', count)

c.execute("SELECT id, task_id, persona_id, prompt, workspace, branch, status, created_at FROM bridge_queue WHERE status='pending' ORDER BY created_at")
rows = c.fetchall()
if not rows:
    print('NO_PENDING_TASKS')
    sys.exit(0)

for r in rows:
    parts = []
    for x in r:
        if x is None:
            parts.append('')
        else:
            parts.append(str(x))
    print('TASK|' + '|'.join(parts))

conn.close()
