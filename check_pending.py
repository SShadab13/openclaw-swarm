import sqlite3
import sys

db_path = r'C:\Users\shada\.kimi_openclaw\workspace\openclaw-swarm\openclaw-swarm.db'
conn = sqlite3.connect(db_path)
cursor = conn.cursor()

cursor.execute("""
    SELECT id, task_id, persona_id, prompt, workspace, branch, status, created_at 
    FROM bridge_queue 
    WHERE status='pending' 
    ORDER BY created_at ASC
""")

rows = cursor.fetchall()
print(f'Pending tasks count: {len(rows)}')
for row in rows:
    print(f"ID={row[0]}, task_id={row[1]}, persona_id={row[2]}, workspace={row[4]}, branch={row[5]}, status={row[6]}, created_at={row[7]}")
    prompt = row[3]
    if len(prompt) > 200:
        print(f"Prompt: {prompt[:200]}...")
    else:
        print(f"Prompt: {prompt}")
    print('---')

conn.close()

# Exit with count as exit code so parent can easily know
sys.exit(len(rows))
