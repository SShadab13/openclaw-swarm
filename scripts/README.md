# Scripts — Knowledge Ingestion Pipeline

_These Python scripts feed the Rust swarm engine. They are the knowledge layer that the Queen queries._

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   ingest.py │────▶│  chunker    │────▶│  embedder   │────▶│  SQLite DB  │
│  (books)    │     │ (Adler L3)  │     │  (Gemini)   │     │ swarm_knowledge.db │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                                                                    │
┌─────────────┐     ┌─────────────┐     ┌─────────────┐            │
│  scrape.py  │────▶│  SearchGraph│────▶│  classify   │─────────────┘
│  (web)      │     │  (multi-src)│     │  (URL type) │
└─────────────┘     └─────────────┘     └─────────────┘
                                            │
                                            ▼
                                    ┌─────────────┐
                                    │  swarm_knowledge.db  │
                                    │  (insights table)      │
                                    └─────────────┘
```

## How They Connect to the Swarm

### `ingest.py` → `Queen::dispatch_with_runner("openclaw", ...)`
- Ingests PDF books into `swarm_knowledge.db` (concepts, chunks, embeddings)
- The Queen can query this via `OpenClawRunner::memory_search()`
- Used by: `architect` persona for architecture decisions, `agentic_ops` for research

### `scrape.py` → `AIScraper` → `KnowledgeIngestor`
- Scrapes web sources (GitHub, arXiv, YouTube) for real-time knowledge
- Saves to `swarm_knowledge.db` insights table
- The Queen dispatches `OpenClawRunner::web_fetch()` to add sources
- Used by: `agentic_ops` persona for staying current, `mlops` for model serving trends

### `swarm_knowledge.db` → `Database` (Rust)
- Rust `db.rs` manages task/agent persistence in `openclaw-swarm.db`
- Python `ingest.py` manages knowledge persistence in `swarm_knowledge.db`
- Both are SQLite, but separate databases by design:
  - `openclaw-swarm.db` = operational (tasks, assignments, letters, diary)
  - `swarm_knowledge.db` = knowledge (books, chunks, concepts, insights)
- The Queen bridges them: operational decisions informed by knowledge queries

## Pipeline Details

### ingest.py
- `collect_books(root)` → finds all PDFs recursively
- `compute_sha256(path)` → deduplication key
- `extract_text(path)` → PyMuPDF extraction
- `chunk_text(text)` → paragraph-level chunks using Adler's analytical reading
- `detect_concepts(chunk)` → definition / key concept / actionable advice detection
- `embed_chunks(chunks)` → Gemini API for semantic vectors
- `save_to_db(...)` → `swarm_knowledge.db` (books, chunks, concepts, embeddings tables)

### scrape.py
- `search_and_scrape(query)` → multi-source web search
- `classify_url(url)` → arXiv / GitHub / blog / video / other
- `save_scraped_insight(...)` → `swarm_knowledge.db` insights table
- `scrape_karpathy_repos()` → specialized scraper for Andrej Karpathy's work

## Integration Points

| Script | Rust Consumer | How |
|--------|--------------|-----|
| `ingest.py` | `OpenClawRunner::memory_search()` | queries `swarm_knowledge.db` |
| `ingest.py` | `Queen::dispatch_task()` | "Research topic X from knowledge base" |
| `scrape.py` | `AIScraper` (concept) | same pattern, different trigger |
| `scrape.py` | `OpenClawRunner::web_fetch()` | adds sources to knowledge base |
| Both | `error_journal.rs` | ingestion failures logged as errors |

## Running

```bash
# Ingest books
python scripts/ingest.py ./books

# Scrape web source
python scripts/scrape.py "agentic AI architectures"

# Both write to: swarm_knowledge.db
```

## Why Python for Ingestion?

Rust is great for the engine (speed, safety, concurrency). Python is better for:
- PyMuPDF (PDF extraction) — no good Rust equivalent
- Gemini API (embeddings) — official Python SDK
- Web scraping (BeautifulSoup, Selenium) — mature ecosystem
- Data science pipelines (chunking, classification)

The boundary is clean: Python produces the knowledge, Rust consumes it.

## Related

- `src/queen.rs` — queries knowledge for task assignment
- `src/runners/openclaw_runner.rs` — `memory_search()` calls knowledge DB
- `src/error_journal.rs` — logs ingestion failures
- `wiki/entities/graphify.md` — how we map the whole system
