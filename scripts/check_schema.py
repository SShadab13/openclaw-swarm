import sqlite3

conn = sqlite3.connect('openclaw-swarm.db')
c = conn.cursor()
c.execute("SELECT sql FROM sqlite_master WHERE type='table' AND name='phases'")
row = c.fetchone()
if row:
    print(row[0])
else:
    print('phases table not found')
conn.close()
