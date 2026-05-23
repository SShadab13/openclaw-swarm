# Swarm Reference Library

**Location:** `openclaw-swarm/docs/books/`

---

## Systems Design

| Book | Author | Year | Size | Status | Notes |
|------|--------|------|------|--------|-------|
| **Designing Data-Intensive Applications** | Martin Kleppmann | 2017 | 6.4 MB | 📥 Transferred | DB schema design, migrations, consistency, reliability. Critical for Phase A DB extensions. |

**Priority chapters for swarm:**
- Chapter 1: Reliable, Scalable, Maintainable Applications
- Chapter 3: Storage and Retrieval (SQLite patterns)
- Chapter 5: Replication (for future cloud scale)
- Chapter 7: Transactions (task state consistency)

---

## Multi-Agent Systems

| Book | Author | Year | Size | Status | Notes |
|------|--------|------|------|--------|-------|
| **An Introduction to MultiAgent Systems** | Michael Wooldridge | 2009 | 10.2 MB | 📥 Transferred | Academic foundation. Agent communication, coordination, game theory, distributed decision-making. |

**Priority chapters for swarm:**
- Chapter 2: Intelligent Agents (definitions, architectures)
- Chapter 8: Communication (agent languages, ACL)
- Chapter 9: Working Together (coordination, cooperation)
- Chapter 11: Methodologies (MAS development process)

---

## Agentic AI (To Acquire)

| Book | Author/Publisher | Year | Status | Notes |
|------|-----------------|------|--------|-------|
| **Building Agentic AI Systems** | Packt / O'Reilly | 2024 | 🔍 Need to acquire | Coordinator/worker/delegator pattern. Multi-step planning. Tool integration. Maps directly to our Queen/Agents/Coordinators. |

**Why we need it:** This is the practical implementation guide. The other two books are foundational; this one is the how-to.

---

## Reading Queue

1. **Now (Phase A prep):** Kleppmann Ch 1, 3, 7 — for DB schema design
2. **Before Phase B:** Wooldridge Ch 8, 9 — for agent communication protocols
3. **Before Phase C:** Acquire + read *Building Agentic AI Systems*
4. **Background:** Swarm knowledge base (8 books already ingested)

---

## How to Ingest New Books

When a new book arrives:
1. Copy to `docs/books/<category>/`
2. Update this index
3. Add to swarm knowledge base: `graphify --directed --wiki`
4. Update `swarm_knowledge.db` with concepts + chunks
5. Reference in design docs with `[[Book Title]]` citations

---

_Last updated: 2026-05-10 by Ayan_
