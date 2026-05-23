#!/usr/bin/env python3
"""Bridge Poller: Check for pending tasks in bridge_queue."""
import sqlite3
import sys

DB_PATH = r'C:\Users\shada\.kimi_openclaw\workspace\openclaw-swarm\openclaw-swarm.db'

def get_pending_tasks():
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    cursor = conn.cursor()
    cursor.execute("""
        SELECT id, task_id, persona_id, prompt, workspace, branch, status, created_at
        FROM bridge_queue
        WHERE status = 'pending'
        ORDER BY created_at ASC
    """)
    rows = cursor.fetchall()
    conn.close()
    return [dict(row) for row in rows]

def mark_started(task_id):
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("""
        UPDATE bridge_queue
        SET status = 'in_progress', started_at = datetime('now')
        WHERE id = ?
    """, (task_id,))
    conn.commit()
    conn.close()

def mark_completed(task_id, result):
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("""
        UPDATE bridge_queue
        SET status = 'completed', completed_at = datetime('now'), result = ?
        WHERE id = ?
    """, (result, task_id))
    conn.commit()
    conn.close()

def mark_failed(task_id, error):
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()
    cursor.execute("""
        UPDATE bridge_queue
        SET status = 'failed', completed_at = datetime('now'), error = ?
        WHERE id = ?
    """, (error, task_id))
    conn.commit()
    conn.close()

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: poller.py <command> [args...]")
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == "list":
        tasks = get_pending_tasks()
        print(f"COUNT: {len(tasks)}")
        for t in tasks:
            print(f"---TASK {t['id']}---")
            print(f"task_id: {t['task_id']}")
            print(f"persona_id: {t['persona_id']}")
            prompt_preview = t['prompt'][:300] + "..." if t['prompt'] and len(t['prompt']) > 300 else t['prompt']
            print(f"prompt: {prompt_preview}")
            print(f"workspace: {t['workspace']}")
            print(f"branch: {t['branch']}")
            print(f"created_at: {t['created_at']}")
    elif cmd == "start" and len(sys.argv) == 3:
        mark_started(sys.argv[2])
        print(f"Marked {sys.argv[2]} as started")
    elif cmd == "complete" and len(sys.argv) == 4:
        mark_completed(sys.argv[2], sys.argv[3])
        print(f"Marked {sys.argv[2]} as completed")
    elif cmd == "fail" and len(sys.argv) == 4:
        mark_failed(sys.argv[2], sys.argv[3])
        print(f"Marked {sys.argv[2]} as failed")
    else:
        print(f"Unknown command: {cmd}")
        sys.exit(1)
