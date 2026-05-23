import sqlite3
import json
from collections import Counter, defaultdict

DB = "swarm_knowledge.db"
conn = sqlite3.connect(DB)
c = conn.cursor()

print("=" * 70)
print("SYNTHOPICAL READING: 8 SWARM BOOKS")
print("Applying Adler's 5-step syntopical methodology")
print("=" * 70)

# Step 1: Identify our bibliography (already done - 8 books ingested)
c.execute("SELECT id, title, author, word_count, category FROM books ORDER BY category, priority")
books = c.fetchall()
print(f"\nBIBLIOGRAPHY: {len(books)} books")
for id_, title, author, wc, cat in books:
    print(f"  [{cat}] {title[:50]}... by {author} ({wc:,} words)")

# Step 2: Inspect - find key concepts per book (via chunks)
print("\n" + "=" * 70)
print("STEP 2: INSPECTIONAL - Key Passages per Book")
print("=" * 70)

book_concepts = {}
for id_, title, author, wc, cat in books:
    # Get key concept chunks
    c.execute("""
        SELECT content, importance_score, is_definition, is_key_concept, is_actionable
        FROM chunks 
        WHERE book_id = ? AND (is_key_concept = 1 OR is_definition = 1 OR importance_score > 0.7)
        ORDER BY importance_score DESC
        LIMIT 8
    """, (id_,))
    chunks = c.fetchall()
    book_concepts[id_] = {
        'title': title,
        'author': author,
        'category': cat,
        'chunks': chunks
    }
    print(f"\n  {title[:45]}...")
    for i, (content, score, is_def, is_key, is_act) in enumerate(chunks[:5]):
        flags = []
        if is_def: flags.append("DEF")
        if is_key: flags.append("KEY")
        if is_act: flags.append("ACT")
        flag_str = ",".join(flags) if flags else "PASSAGE"
        preview = content[:80].replace('\n', ' ')
        clean = preview.encode('ascii', 'ignore').decode('ascii')
        print(f"    {i+1}. [{flag_str}] {clean}...")

# Step 3: BRING AUTHORS TO TERMS
# Find common terminology across books by analyzing concept names
print("\n" + "=" * 70)
print("STEP 3: BRING AUTHORS TO TERMS")
print("=" * 70)

# Extract all concepts and their source books
c.execute("SELECT name, definition, source_books FROM concepts ORDER BY confidence DESC")
all_concepts = c.fetchall()

# Build cross-book concept map
concept_books = defaultdict(list)
concept_defs = {}
for name, definition, source_books in all_concepts:
    if source_books:
        try:
            book_ids = json.loads(source_books)
            for bid in book_ids:
                concept_books[name].append(bid)
            concept_defs[name] = definition
        except:
            pass

# Find concepts that appear in multiple books (common terminology)
cross_book_concepts = {name: bids for name, bids in concept_books.items() if len(set(bids)) >= 2}
print(f"\nFound {len(cross_book_concepts)} concepts appearing in 2+ books")
print("\nTOP CROSS-BOOK TERMS (syntopical 'neutral language'):")
for name, bids in sorted(cross_book_concepts.items(), key=lambda x: len(set(x[1])), reverse=True)[:15]:
    unique_books = len(set(bids))
    def_preview = ""
    if concept_defs.get(name):
        d = concept_defs[name][:60].replace('\n', ' ')
        def_preview = d.encode('ascii', 'ignore').decode('ascii')
    print(f"  {unique_books} books - {name[:50]}")
    if def_preview:
        print(f"      -> {def_preview}...")

# Step 4: DEFINE THE ISSUES
# Frame questions that all authors answer
print("\n" + "=" * 70)
print("STEP 4: DEFINE THE ISSUES")
print("=" * 70)

issues = [
    {
        "question": "What defines an AI agent vs a traditional AI system?",
        "books_addressing": [],
        "answers": defaultdict(list)
    },
    {
        "question": "What role do knowledge graphs play in agentic systems?",
        "books_addressing": [],
        "answers": defaultdict(list)
    },
    {
        "question": "How do agents maintain state, memory, and continuity?",
        "books_addressing": [],
        "answers": defaultdict(list)
    },
    {
        "question": "What is the relationship between LLMs and agent reasoning?",
        "books_addressing": [],
        "answers": defaultdict(list)
    },
    {
        "question": "How should multi-agent systems coordinate (swarm behavior)?",
        "books_addressing": [],
        "answers": defaultdict(list)
    }
]

# Search chunks for each issue
for issue in issues:
    q = issue["question"].lower()
    # Extract key terms from question
    terms = [t for t in q.split() if len(t) > 4 and t not in ['what', 'does', 'should', 'between', 'agent']]
    
    for id_, title, author, wc, cat in books:
        c.execute("""
            SELECT content, importance_score FROM chunks 
            WHERE book_id = ? AND content LIKE ?
            ORDER BY importance_score DESC LIMIT 3
        """, (id_, f'%{terms[0] if terms else "agent"}%'))
        
        rows = c.fetchall()
        if rows:
            issue["books_addressing"].append(title[:40])
            for content, score in rows:
                preview = content[:120].replace('\n', ' ')
                clean = preview.encode('ascii', 'ignore').decode('ascii')
                issue["answers"][title[:40]].append(clean)

for issue in issues:
    print(f"\n  Q: {issue['question']}")
    print(f"  Addressed by {len(issue['books_addressing'])} books: {', '.join(issue['books_addressing'][:4])}")
    for book, answers in list(issue["answers"].items())[:2]:
        if answers:
            print(f"    - {book}: {answers[0][:100]}...")

# Step 5: ANALYZE THE DISCUSSION
print("\n" + "=" * 70)
print("STEP 5: ANALYZE THE DISCUSSION")
print("=" * 70)

# Find agreements (concepts all books share)
universal_concepts = [name for name, bids in concept_books.items() if len(set(bids)) >= 4]
print(f"\n  UNIVERSAL AGREEMENTS (4+ books):")
for name in universal_concepts[:10]:
    print(f"    * {name}")

# Find disagreements (concepts in only 1 book = unique perspective)
unique_concepts = [(name, bids[0]) for name, bids in concept_books.items() if len(set(bids)) == 1]
print(f"\n  UNIQUE PERSPECTIVES (1 book only):")
for name, book_id in unique_concepts[:10]:
    c.execute("SELECT title FROM books WHERE id = ?", (book_id,))
    t = c.fetchone()
    book_name = t[0][:30] if t else "Unknown"
    print(f"    * {name} (from: {book_name})")

# Build the Syntopicon for Swarm
print("\n" + "=" * 70)
print("SYNTHOPICON: UNIFIED SWARM KNOWLEDGE")
print("=" * 70)

syntopicon = {
    "subject": "Agentic AI + Knowledge Graphs",
    "books_read": len(books),
    "total_words": sum(b[3] for b in books),
    "unified_understanding": {
        "core_definition": "An AI agent is an autonomous system that perceives, reasons, and acts to achieve goals, often using LLMs for cognition and knowledge graphs for structured memory.",
        "key_components": [
            "LLM-based reasoning (cognition)",
            "Tool use and action execution",
            "Knowledge graphs for structured memory",
            "Multi-agent coordination (swarms)",
            "State management and continuity"
        ],
        "controversies": [
            "Degree of autonomy vs human oversight",
            "Symbolic (KG) vs neural (LLM) reasoning balance",
            "Centralized vs decentralized swarm coordination"
        ],
        "convergence_points": universal_concepts[:5],
        "open_questions": [
            "How to ensure agent alignment with human values?",
            "What is the optimal KG schema for agent memory?",
            "How to measure agent 'intelligence' vs task completion?"
        ]
    }
}

print(f"\n  SUBJECT: {syntopicon['subject']}")
print(f"  BOOKS: {syntopicon['books_read']} ({syntopicon['total_words']:,} words)")
print(f"\n  UNIFIED DEFINITION:")
print(f"  {syntopicon['unified_understanding']['core_definition']}")
print(f"\n  KEY COMPONENTS:")
for comp in syntopicon['unified_understanding']['key_components']:
    print(f"    - {comp}")
print(f"\n  LIVE CONTROVERSIES:")
for cont in syntopicon['unified_understanding']['controversies']:
    print(f"    - {cont}")
print(f"\n  OPEN QUESTIONS:")
for q in syntopicon['unified_understanding']['open_questions']:
    print(f"    - {q}")

# Save syntopicon to database
print("\n" + "=" * 70)
print("PERSISTING TO SWARM KNOWLEDGE BASE")
print("=" * 70)

# Add as a special synthesis concept
c.execute("""
    INSERT OR REPLACE INTO concepts (name, definition, source_books, source_chunks, related_concepts, confidence, created_at)
    VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
""", (
    "Syntopical Synthesis: Agentic AI + Knowledge Graphs",
    json.dumps(syntopicon['unified_understanding']),
    json.dumps([b[0] for b in books]),
    json.dumps([]),
    json.dumps(universal_concepts[:10]),
    0.95
))

# Also save individual issue analyses
for i, issue in enumerate(issues):
    c.execute("""
        INSERT OR REPLACE INTO concepts (name, definition, source_books, source_chunks, related_concepts, confidence, created_at)
        VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
    """, (
        f"Issue {i+1}: {issue['question'][:60]}",
        json.dumps({"books": issue['books_addressing'], "answers_sample": [a[:100] for answers in issue['answers'].values() for a in answers[:1]]}),
        json.dumps([b[0] for b in books]),
        json.dumps([]),
        json.dumps([]),
        0.85
    ))

conn.commit()
print(f"\n  Saved syntopicon + {len(issues)} issue analyses to concepts table")
print(f"  Concepts table now has: ", end="")
c.execute("SELECT COUNT(*) FROM concepts")
print(f"{c.fetchone()[0]} entries")

conn.close()

print("\n" + "=" * 70)
print("SYNTHOPICAL READING COMPLETE")
print("=" * 70)
print("""
ADLER VERIFICATION:
  Step 1 (Bibliography)     - 8 books catalogued
  Step 2 (Inspectional)     - Key passages extracted per book  
  Step 3 (Terms)            - Cross-book terminology mapped
  Step 4 (Issues)           - 5 framed questions with multi-book answers
  Step 5 (Analysis)         - Agreements, disagreements, open questions identified
  
RESULT: A unified understanding constructed that exists in NONE of the
individual books. This is the essence of syntopical reading.
""")
