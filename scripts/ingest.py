#!/usr/bin/env python3
"""
Knowledge Ingestion Pipeline for OpenClaw Swarm
Converts PDF books → text chunks → SQLite knowledge base
"""

import sqlite3
import json
import re
import os
import sys
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Optional, Tuple
import hashlib

# Try to import PDF libraries
try:
    import fitz  # PyMuPDF
    HAS_PYMUPDF = True
except ImportError:
    HAS_PYMUPDF = False
    print("⚠️ PyMuPDF not installed. Install with: pip install pymupdf")

try:
    import pdfplumber
    HAS_PDFPLUMBER = True
except ImportError:
    HAS_PDFPLUMBER = False

class KnowledgeIngestor:
    """Ingests PDF books into swarm knowledge base."""
    
    def __init__(self, db_path: str = "swarm_knowledge.db"):
        self.db_path = db_path
        self.init_db()
    
    def init_db(self):
        """Initialize knowledge database."""
        conn = sqlite3.connect(self.db_path)
        c = conn.cursor()
        
        # Books table
        c.execute('''
            CREATE TABLE IF NOT EXISTS books (
                id INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                author TEXT,
                filepath TEXT UNIQUE,
                file_hash TEXT,
                total_pages INTEGER,
                word_count INTEGER,
                status TEXT DEFAULT 'pending',
                tags TEXT,
                category TEXT,
                priority INTEGER DEFAULT 5,
                created_at TEXT,
                ingested_at TEXT,
                four_questions TEXT
            )
        ''')
        
        # Chunks table (paragraph-level)
        c.execute('''
            CREATE TABLE IF NOT EXISTS chunks (
                id INTEGER PRIMARY KEY,
                book_id INTEGER,
                chapter TEXT,
                page_start INTEGER,
                page_end INTEGER,
                content TEXT,
                word_count INTEGER,
                chunk_hash TEXT UNIQUE,
                tags TEXT,
                importance_score REAL DEFAULT 0.0,
                is_definition BOOLEAN DEFAULT 0,
                is_key_concept BOOLEAN DEFAULT 0,
                is_actionable BOOLEAN DEFAULT 0,
                created_at TEXT,
                FOREIGN KEY (book_id) REFERENCES books(id)
            )
        ''')
        
        # Concepts table (extracted key concepts)
        c.execute('''
            CREATE TABLE IF NOT EXISTS concepts (
                id INTEGER PRIMARY KEY,
                name TEXT UNIQUE,
                definition TEXT,
                source_books TEXT,
                source_chunks TEXT,
                related_concepts TEXT,
                confidence REAL DEFAULT 1.0,
                created_at TEXT
            )
        ''')
        
        # Syntopical links (cross-book connections)
        c.execute('''
            CREATE TABLE IF NOT EXISTS syntopical_links (
                id INTEGER PRIMARY KEY,
                concept_a TEXT,
                concept_b TEXT,
                book_a_id INTEGER,
                book_b_id INTEGER,
                relationship_type TEXT,
                evidence TEXT,
                created_at TEXT
            )
        ''')
        
        # FTS (Full Text Search) for fast querying
        c.execute('''
            CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
                content,
                tags,
                book_id UNINDEXED,
                tokenize='porter'
            )
        ''')
        
        conn.commit()
        conn.close()
        print(f"✅ Knowledge database initialized: {self.db_path}")
    
    def compute_hash(self, filepath: str) -> str:
        """Compute SHA256 hash of file."""
        h = hashlib.sha256()
        with open(filepath, 'rb') as f:
            for chunk in iter(lambda: f.read(8192), b''):
                h.update(chunk)
        return h.hexdigest()[:16]
    
    def extract_text_pymupdf(self, filepath: str) -> Tuple[str, int]:
        """Extract text using PyMuPDF."""
        doc = fitz.open(filepath)
        text_parts = []
        for page in doc:
            text_parts.append(page.get_text())
        doc.close()
        return "\n".join(text_parts), len(text_parts)
    
    def chunk_text(self, text: str, max_chunk_size: int = 800, overlap: int = 100) -> List[Dict]:
        """
        Chunk text into paragraph-level segments.
        Uses Adler's analytical reading structure:
        - Detect chapter boundaries
        - Identify key concepts (bold, definitions)
        - Preserve argument sequences
        """
        chunks = []
        
        # Split by paragraphs (double newline)
        paragraphs = re.split(r'\n\s*\n', text)
        
        current_chunk = []
        current_words = 0
        chapter = "Unknown"
        page_num = 0
        
        for para in paragraphs:
            para = para.strip()
            if not para:
                continue
            
            # Detect chapter headings
            chapter_match = re.match(r'^(Chapter\s+\d+|\d+\.|\d+\.\d+\.?)\s+(.+)', para, re.IGNORECASE)
            if chapter_match or (len(para) < 100 and para.isupper()):
                # Save current chunk before new chapter
                if current_chunk:
                    chunk_text = "\n\n".join(current_chunk)
                    chunks.append({
                        'content': chunk_text,
                        'chapter': chapter,
                        'word_count': current_words,
                        'is_definition': self._is_definition(chunk_text),
                        'is_key_concept': self._is_key_concept(chunk_text),
                        'is_actionable': self._is_actionable(chunk_text),
                    })
                chapter = para[:100]
                current_chunk = []
                current_words = 0
                continue
            
            word_count = len(para.split())
            
            # If adding this paragraph exceeds chunk size, save current and start new
            if current_words + word_count > max_chunk_size and current_chunk:
                chunk_text = "\n\n".join(current_chunk)
                chunks.append({
                    'content': chunk_text,
                    'chapter': chapter,
                    'word_count': current_words,
                    'is_definition': self._is_definition(chunk_text),
                    'is_key_concept': self._is_key_concept(chunk_text),
                    'is_actionable': self._is_actionable(chunk_text),
                })
                
                # Overlap: keep last paragraph for context
                if overlap > 0 and len(current_chunk) > 1:
                    last_para = current_chunk[-1]
                    current_chunk = [last_para, para]
                    current_words = len(last_para.split()) + word_count
                else:
                    current_chunk = [para]
                    current_words = word_count
            else:
                current_chunk.append(para)
                current_words += word_count
        
        # Don't forget last chunk
        if current_chunk:
            chunk_text = "\n\n".join(current_chunk)
            chunks.append({
                'content': chunk_text,
                'chapter': chapter,
                'word_count': current_words,
                'is_definition': self._is_definition(chunk_text),
                'is_key_concept': self._is_key_concept(chunk_text),
                'is_actionable': self._is_actionable(chunk_text),
            })
        
        return chunks
    
    def _is_definition(self, text: str) -> bool:
        """Detect if text contains a definition."""
        patterns = [
            r'\b(is defined as|is a|refers to|means|consists of)\b',
            r'\b[A-Z][a-zA-Z\s]+:\s*',
            r'\b(definition|define)\b',
        ]
        return any(re.search(p, text, re.IGNORECASE) for p in patterns)
    
    def _is_key_concept(self, text: str) -> bool:
        """Detect if text contains a key concept."""
        patterns = [
            r'\b(key|important|crucial|essential|fundamental|core)\b',
            r'\b(principle|theorem|law|model|framework|pattern)\b',
            r'\b(remember|note that|critical|vital)\b',
        ]
        return any(re.search(p, text, re.IGNORECASE) for p in patterns)
    
    def _is_actionable(self, text: str) -> bool:
        """Detect if text contains actionable advice."""
        patterns = [
            r'\b(how to|steps?|guide|tutorial|implement|build|create)\b',
            r'\b(step \d+|first|second|third|then|next|finally)\b',
            r'\b(practice|exercise|do this|try|experiment)\b',
        ]
        return any(re.search(p, text, re.IGNORECASE) for p in patterns)
    
    def extract_concepts(self, chunk: Dict, book_title: str) -> List[Dict]:
        """Extract named concepts from a chunk."""
        concepts = []
        text = chunk['content']
        
        # Pattern: "X is a Y" or "X refers to Y"
        concept_patterns = [
            r'\b([A-Z][a-zA-Z\s]{2,30})\s+is\s+(?:a|an|the)\s+([^.]+)',
            r'\b([A-Z][a-zA-Z\s]{2,30})\s+refers to\s+([^.]+)',
            r'\b(The [A-Z][a-zA-Z\s]{2,30})\s+(?:is|are)\s+([^.]+)',
        ]
        
        for pattern in concept_patterns:
            for match in re.finditer(pattern, text):
                name = match.group(1).strip()
                definition = match.group(2).strip()[:200]
                if len(name) > 3 and len(definition) > 10:
                    concepts.append({
                        'name': name,
                        'definition': definition,
                        'source': book_title,
                    })
        
        return concepts
    
    def ingest_book(self, filepath: str, title: str = None, author: str = None,
                    category: str = None, priority: int = 5, tags: List[str] = None) -> int:
        """
        Ingest a single book into knowledge base.
        
        Implements Adler's Four Levels:
        1. Inspectional: Extract TOC, determine worth
        2. Analytical: Deep chunk, classify, outline
        3. Syntopical: Extract concepts, link to existing knowledge
        """
        print(f"\n📖 Ingesting: {title or filepath}")
        
        if not os.path.exists(filepath):
            print(f"❌ File not found: {filepath}")
            return -1
        
        if not HAS_PYMUPDF:
            print("❌ PyMuPDF required. Install: pip install pymupdf")
            return -1
        
        # Compute file hash
        file_hash = self.compute_hash(filepath)
        
        conn = sqlite3.connect(self.db_path)
        c = conn.cursor()
        
        # Check if already ingested
        c.execute("SELECT id, status FROM books WHERE file_hash = ?", (file_hash,))
        existing = c.fetchone()
        if existing and existing[1] == 'ingested':
            print(f"⏭️ Already ingested (hash match). ID: {existing[0]}")
            conn.close()
            return existing[0]
        
        # Extract text
        print("  🔍 Extracting text...")
        text, page_count = self.extract_text_pymupdf(filepath)
        word_count = len(text.split())
        
        # Insert book record
        now = datetime.now().isoformat()
        c.execute('''
            INSERT OR REPLACE INTO books
            (title, author, filepath, file_hash, total_pages, word_count, status, tags, category, priority, created_at, ingested_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ''', (
            title or Path(filepath).stem,
            author or 'Unknown',
            filepath,
            file_hash,
            page_count,
            word_count,
            'ingesting',
            json.dumps(tags or []),
            category or 'uncategorized',
            priority,
            now,
            now,
        ))
        
        book_id = c.lastrowid
        
        # Chunk the text
        print(f"  ✂️ Chunking {word_count} words...")
        chunks = self.chunk_text(text)
        print(f"  → {len(chunks)} chunks created")
        
        # Store chunks
        all_concepts = []
        for i, chunk in enumerate(chunks):
            chunk_hash = hashlib.sha256(chunk['content'].encode()).hexdigest()[:16]
            
            c.execute('''
                INSERT OR IGNORE INTO chunks
                (book_id, chapter, content, word_count, chunk_hash, tags,
                 is_definition, is_key_concept, is_actionable, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ''', (
                book_id,
                chunk['chapter'],
                chunk['content'],
                chunk['word_count'],
                chunk_hash,
                json.dumps(tags or []),
                chunk['is_definition'],
                chunk['is_key_concept'],
                chunk['is_actionable'],
                now,
            ))
            
            # Also insert into FTS
            c.execute('''
                INSERT INTO chunks_fts (content, tags, book_id)
                VALUES (?, ?, ?)
            ''', (chunk['content'], json.dumps(tags or []), book_id))
            
            # Extract concepts from chunk
            chunk_concepts = self.extract_concepts(chunk, title or Path(filepath).stem)
            all_concepts.extend(chunk_concepts)
        
        # Store unique concepts
        seen_concepts = set()
        for concept in all_concepts:
            name = concept['name'].lower()
            if name not in seen_concepts and len(name) > 3:
                seen_concepts.add(name)
                c.execute('''
                    INSERT OR IGNORE INTO concepts (name, definition, source_books, created_at)
                    VALUES (?, ?, ?, ?)
                ''', (
                    concept['name'],
                    concept['definition'],
                    json.dumps([concept['source']]),
                    now,
                ))
        
        # Mark as ingested
        c.execute("UPDATE books SET status = 'ingested' WHERE id = ?", (book_id,))
        
        # Answer the Four Questions
        four_questions = self._answer_four_questions(text, title or Path(filepath).stem)
        c.execute("UPDATE books SET four_questions = ? WHERE id = ?",
                  (json.dumps(four_questions), book_id))
        
        conn.commit()
        conn.close()
        
        print(f"  ✅ Ingested! Book ID: {book_id}")
        print(f"  📊 {page_count} pages | {word_count} words | {len(chunks)} chunks | {len(seen_concepts)} concepts")
        
        return book_id
    
    def _answer_four_questions(self, text: str, title: str) -> Dict:
        """Answer Adler's Four Questions about the book."""
        # Simple heuristic answers
        first_1000 = text[:1000]
        
        # Q1: What is this book about?
        about = title
        
        # Q2: What is being said in detail?
        detail = f"Book contains approximately {len(text.split())} words"
        
        # Q3: Is this book true?
        # Heuristic: check for citations, references
        has_citations = bool(re.search(r'\[\d+\]|\(\d{4}\)|et al\.|references|bibliography', text, re.IGNORECASE))
        
        # Q4: What of it?
        # Heuristic: check for actionable content
        has_actionable = bool(re.search(r'how to|steps?|guide|implement|build|create|practice', text, re.IGNORECASE))
        
        return {
            'what_is_it_about': about,
            'what_is_said': detail,
            'is_it_true': 'Has citations' if has_citations else 'Check sources manually',
            'what_of_it': 'Actionable' if has_actionable else 'Informational',
        }
    
    def search_knowledge(self, query: str, limit: int = 10) -> List[Dict]:
        """Search knowledge base using FTS."""
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        c = conn.cursor()
        
        c.execute('''
            SELECT c.*, b.title as book_title
            FROM chunks_fts f
            JOIN chunks c ON f.rowid = c.id
            JOIN books b ON c.book_id = b.id
            WHERE chunks_fts MATCH ?
            ORDER BY rank
            LIMIT ?
        ''', (query, limit))
        
        results = [dict(row) for row in c.fetchall()]
        conn.close()
        return results
    
    def get_book_summary(self, book_id: int) -> Dict:
        """Get summary of ingested book."""
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        c = conn.cursor()
        
        c.execute("SELECT * FROM books WHERE id = ?", (book_id,))
        book = dict(c.fetchone()) if c.fetchone else None
        
        if book:
            c.execute("SELECT COUNT(*) FROM chunks WHERE book_id = ?", (book_id,))
            book['chunk_count'] = c.fetchone()[0]
            
            c.execute("SELECT COUNT(*) FROM concepts WHERE json_extract(source_books, '$') LIKE ?",
                      (f'%"{book["title"]}"%',))
            book['concept_count'] = c.fetchone()[0]
        
        conn.close()
        return book or {}


def main():
    """CLI for knowledge ingestion."""
    import argparse
    
    parser = argparse.ArgumentParser(description='Swarm Knowledge Ingestion Pipeline')
    parser.add_argument('--db', default='swarm_knowledge.db', help='Database path')
    parser.add_argument('command', choices=['ingest', 'search', 'summary', 'list'])
    parser.add_argument('args', nargs='*')
    
    args = parser.parse_args()
    
    ingestor = KnowledgeIngestor(args.db)
    
    if args.command == 'ingest':
        if not args.args:
            print("Usage: python ingest.py ingest <filepath> [title] [author] [category] [priority]")
            sys.exit(1)
        
        filepath = args.args[0]
        title = args.args[1] if len(args.args) > 1 else None
        author = args.args[2] if len(args.args) > 2 else None
        category = args.args[3] if len(args.args) > 3 else None
        priority = int(args.args[4]) if len(args.args) > 4 else 5
        
        ingestor.ingest_book(filepath, title, author, category, priority)
    
    elif args.command == 'search':
        if not args.args:
            print("Usage: python ingest.py search <query>")
            sys.exit(1)
        
        query = args.args[0]
        results = ingestor.search_knowledge(query)
        
        print(f"\n🔍 Results for: '{query}'")
        for r in results:
            print(f"\n  📖 {r.get('book_title', 'Unknown')}")
            print(f"  🏷️  {r.get('chapter', 'Unknown chapter')}")
            preview = r['content'][:200] if r['content'] else ''
            print(f"  📝 {preview}...")
    
    elif args.command == 'summary':
        if not args.args:
            print("Usage: python ingest.py summary <book_id>")
            sys.exit(1)
        
        book_id = int(args.args[0])
        summary = ingestor.get_book_summary(book_id)
        
        print(f"\n📊 Book Summary (ID: {book_id})")
        for k, v in summary.items():
            print(f"  {k}: {v}")
    
    elif args.command == 'list':
        conn = sqlite3.connect(args.db)
        conn.row_factory = sqlite3.Row
        c = conn.cursor()
        
        c.execute("SELECT id, title, author, status, category, priority FROM books ORDER BY priority")
        books = c.fetchall()
        
        print("\n📚 Ingested Books:")
        for b in books:
            status_emoji = '✅' if b['status'] == 'ingested' else '⏳'
            print(f"  {status_emoji} [{b['priority']}] {b['title']} by {b['author']} ({b['category']})")
        
        conn.close()


if __name__ == '__main__':
    main()
