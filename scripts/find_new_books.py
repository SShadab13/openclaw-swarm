import sqlite3
import os
from pathlib import Path

# Get all PDFs in My Drive/Books
pdf_paths = []
for root, dirs, files in os.walk(r'I:\My Drive\Books'):
    for f in files:
        if f.lower().endswith('.pdf'):
            pdf_paths.append(os.path.join(root, f))

# Get ingested books from DB
conn = sqlite3.connect('swarm_knowledge.db')
c = conn.cursor()
c.execute('SELECT filepath FROM books')
ingested = set(row[0] for row in c.fetchall())
conn.close()

# Find new books
new_books = [p for p in pdf_paths if p not in ingested]

print(f'Total PDFs found: {len(pdf_paths)}')
print(f'Already ingested: {len(ingested)}')
print(f'New books to ingest: {len(new_books)}')
print()

if new_books:
    print('New books:')
    for p in new_books[:20]:
        size_mb = round(os.path.getsize(p) / (1024*1024), 2)
        print(f'  - {p} ({size_mb} MB)')
    if len(new_books) > 20:
        print(f'  ... and {len(new_books) - 20} more')
