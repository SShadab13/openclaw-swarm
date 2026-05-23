import sqlite3
conn = sqlite3.connect('openclaw-swarm.db')
c = conn.cursor()
c.execute("UPDATE tasks SET status = ? WHERE id = ?", ('completed', '0c192311-1bef-4175-9e22-a850332b34ef'))
conn.commit()
print('Task marked completed')
conn.close()
