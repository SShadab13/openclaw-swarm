import sqlite3
import sys

db_path = r"C:\Users\shada\.kimi_openclaw\workspace\openclaw-swarm\openclaw-swarm.db"

conn = sqlite3.connect(db_path)
conn.row_factory = sqlite3.Row
cursor = conn.cursor()

cursor.execute("SELECT id, task_id, persona_id, prompt, workspace, branch, status, created_at FROM bridge_queue WHERE status='pending' ORDER BY created_at")
rows = cursor.fetchall()

print(f"PENDING_COUNT:{len(rows)}")
for row in rows:
    print(f"---TASK_START---")
    print(f"ID:{row['id']}")
    print(f"TASK_ID:{row['task_id']}")
    print(f"PERSONA_ID:{row['persona_id']}")
    print(f"PROMPT:{row['prompt']}")
    print(f"WORKSPACE:{row['workspace']}")
    print(f"BRANCH:{row['branch']}")
    print(f"STATUS:{row['status']}")
    print(f"CREATED_AT:{row['created_at']}")
    print(f"---TASK_END---")

conn.close()
