import sqlite3

conn = sqlite3.connect('swarm_knowledge.db')
c = conn.cursor()

c.execute('SELECT id, title, author, total_pages, word_count, ingested_at FROM books ORDER BY id')
for b in c.fetchall():
    print(f'{b[0]}. {b[1]} by {b[2]} ({b[3]} pages, {b[4]} words) - {b[5]}')

c.execute('SELECT COUNT(*) FROM books')
print(f'\nTotal: {c.fetchone()[0]} books')

c.execute('SELECT COUNT(*) FROM chunks')
print(f'Chunks: {c.fetchone()[0]}')

c.execute('SELECT COUNT(*) FROM concepts')
print(f'Concepts: {c.fetchone()[0]}')

conn.close()
