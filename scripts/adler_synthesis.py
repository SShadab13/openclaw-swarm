import sqlite3
import json

conn = sqlite3.connect('swarm_knowledge.db')
c = conn.cursor()

# Get all books with their 4 questions (Adler framework)
print("=" * 70)
print("ADLER'S ANALYTICAL READING - 8 BOOK SYNTHESIS")
print("=" * 70)

c.execute('''
    SELECT id, title, author, total_pages, word_count,
           four_questions, category, priority
    FROM books ORDER BY priority, id
''')

books = c.fetchall()
for b in books:
    id, title, author, pages, words, fq_json, cat, priority = b
    fq = json.loads(fq_json) if fq_json else {}
    print(f"\n{'-' * 70}")
    print(f"[BOOK] {title}")
    print(f"   by {author} | {pages} pages | {words:,} words | Priority: {priority}")
    print(f"   Category: {cat}")
    if fq:
        print(f"\n   Adler's 4 Questions:")
        print(f"   1. What is it about? -> {fq.get('what_is_it_about', 'N/A')}")
        print(f"   2. What is being said? -> {fq.get('what_is_said', 'N/A')[:80]}...")
        print(f"   3. Is it true? -> {fq.get('is_it_true', 'N/A')}")
        print(f"   4. What of it? -> {fq.get('what_of_it', 'N/A')}")

# Count concepts by book
print(f"\n{'=' * 70}")
print("CONCEPTS BY BOOK")
print("=" * 70)

for b in books:
    id, title, _, _, _, _, _, _ = b
    c.execute('SELECT COUNT(*) FROM chunks WHERE book_id = ?', (id,))
    count = c.fetchone()[0]
    # Sample concepts from this book
    c.execute('''
        SELECT ch.content FROM chunks ch 
        WHERE ch.book_id = ? AND ch.is_key_concept = 1 
        LIMIT 5
    ''', (id,))
    sample = c.fetchall()
    print(f"\n{title[:50]}... -> {count} chunks")
    for row in sample:
        content = row[0][:60].encode('ascii', 'ignore').decode('ascii')
        print(f"   * {content}...")

# Top concepts across all books
print("\n" + "=" * 70)
print("TOP CONCEPTS (cross-book)")
print("=" * 70)

c.execute('''
    SELECT name, definition, source_books
    FROM concepts 
    ORDER BY confidence DESC 
    LIMIT 15
''')

for name, definition, source_books in c.fetchall():
    books = json.loads(source_books) if source_books else []
    clean_name = name[:50].encode('ascii', 'ignore').decode('ascii')
    print(f"   {len(books)} books - {clean_name}")
    if definition:
        clean_def = definition[:70].encode('ascii', 'ignore').decode('ascii')
        print(f"      {clean_def}...")

# Chunk statistics
print(f"\n{'=' * 70}")
print("KNOWLEDGE CHUNKS")
print("=" * 70)

c.execute('SELECT COUNT(*) FROM chunks')
print(f"Total chunks: {c.fetchone()[0]:,}")

c.execute('''
    SELECT b.title, COUNT(ch.id) as chunks
    FROM chunks ch
    JOIN books b ON ch.book_id = b.id
    GROUP BY b.id
    ORDER BY chunks DESC
''')

for title, chunks in c.fetchall():
    print(f"   {chunks:4d} chunks - {title[:50]}")

conn.close()
print("\n" + "=" * 70)
print("SYNTHESIS COMPLETE - Ready for swarm integration")
print("=" * 70)
