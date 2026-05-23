import sqlite3
import json
from datetime import datetime, timezone

db_path = r"C:\Users\shada\.kimi_openclaw\workspace\openclaw-swarm\openclaw-swarm.db"

def mark_started(conn, task_id):
    cursor = conn.cursor()
    now = datetime.now(timezone.utc).isoformat()
    cursor.execute(
        "UPDATE bridge_queue SET status='processing', started_at=? WHERE id=?",
        (now, task_id)
    )
    conn.commit()

def mark_completed(conn, task_id, result):
    cursor = conn.cursor()
    now = datetime.now(timezone.utc).isoformat()
    cursor.execute(
        "UPDATE bridge_queue SET status='completed', completed_at=?, result=? WHERE id=?",
        (now, result, task_id)
    )
    conn.commit()

def mark_failed(conn, task_id, error):
    cursor = conn.cursor()
    now = datetime.now(timezone.utc).isoformat()
    cursor.execute(
        "UPDATE bridge_queue SET status='failed', completed_at=?, error=? WHERE id=?",
        (now, error, task_id)
    )
    conn.commit()

conn = sqlite3.connect(db_path)
conn.row_factory = sqlite3.Row
cursor = conn.cursor()

# Fetch pending tasks
cursor.execute("SELECT id, task_id, persona_id, prompt, workspace, branch, status, created_at FROM bridge_queue WHERE status='pending' ORDER BY created_at")
pending = cursor.fetchall()

if not pending:
    print("No pending tasks.")
    conn.close()
    exit(0)

print(f"Found {len(pending)} pending task(s).")

for row in pending:
    task_db_id = row['id']
    task_id = row['task_id']
    persona_id = row['persona_id']
    prompt = row['prompt']
    workspace = row['workspace']
    branch = row['branch']
    
    print(f"Processing task {task_db_id} (task_id={task_id}, persona={persona_id})")
    mark_started(conn, task_db_id)
    
    # TODO: Use sessions_spawn to create subagent with the prompt
    # For now, mark as failed since we are in the cron bridge poller
    # and actual sessions_spawn should be called by the outer agent.
    
conn.close()
