---
title: "Mental Models × Karpathy Principles — Agent Development Synthesis"
type: synthesis
tags: [mental-models, karpathy, coding, swarm, first-principles]
sources:
  - "Shane Parrish — Great Mental Models Vol 1 (General Thinking Concepts)"
  - "Andrej Karpathy — GitHub repos (nanoGPT, micrograd, build-nanogpt, llama2.c, minGPT)"
  - "Andrej Karpathy — LLM Coding Guidelines (X thread, 2026)"
created: 2026-05-09
updated: 2026-05-09
---

# Mental Models × Karpathy Principles

## The Core Insight

Karpathy's coding philosophy IS a practical application of Farnam Street's mental models. Every principle in his repos maps directly to a mental model. Understanding both layers makes you a better agent developer.

---

## Model-to-Principle Mapping

| Mental Model | Karpathy Principle | Swarm Application |
|---|---|---|
| **First Principles Thinking** | Reproduce from scratch (build-nanogpt from empty file) | Build features from fundamental units, not copy-paste |
| **Occam's Razor** | Simplicity First (~300 line nanoGPT, ~100 line micrograd) | Minimum code. No speculative abstractions. |
| **The Map is not the Territory** | State assumptions explicitly | Our abstractions (ORM, framework) aren't the real system |
| **Circle of Competence** | If uncertain, ask; don't hide confusion | Know what you don't know. Flag gaps. |
| **Second-Order Thinking** | Goal-driven execution with verifiable criteria | Think about consequences of consequences |
| **Inversion** | "Write test that reproduces bug, then make it pass" | Ask "what would make this fail?" first |
| **Falsifiability** | Define success criteria before coding | Every claim must be testable. No hand-waving. |
| **Probabilistic Thinking** | Surface tradeoffs, don't pick silently | Multiple interpretations exist. Present them. |
| **Causation vs. Correlation** | Don't "improve" adjacent code, match existing style | What works ≠ why it works. Respect proven patterns. |
| **Hanlon's Razor** | Don't refactor things that aren't broken | Assume incompetence, not malice. Don't fix what works. |

---

## Volume 1: General Thinking Concepts (12 Models)

### Core Models (9)
1. **The Map is not the Territory** — Our representations are simplified. Don't confuse the model with reality.
2. **Circle of Competence** — Know your edge. When you don't know, say so.
3. **First Principles Thinking** — Break to fundamentals. Build up. (Tesla battery cost, SpaceX reusable rockets)
4. **Thought Experiment** — Simulate scenarios mentally before executing.
5. **Second-Order Thinking** — Consequences have consequences. Think one level deeper.
6. **Probabilistic Thinking** — Nothing is certain. Assign probabilities, update with evidence.
7. **Inversion** — Instead of "how to succeed?" ask "how to fail?" Then avoid those.
8. **Occam's Razor** — The simplest explanation is usually correct. Prefer simple solutions.
9. **Hanlon's Razor** — Never attribute to malice what can be explained by incompetence.

### Supporting Ideas (3)
10. **Falsifiability** — A claim is only scientific if it can be proven wrong. Define what would disprove your approach.
11. **Necessity and Sufficiency** — Check if conditions are required (necessary) and/or enough (sufficient).
12. **Causation vs. Correlation** — Just because A and B happen together doesn't mean A causes B.

---

## Karpathy Repo Analysis

### nanoGPT (~300 lines train.py + ~300 lines model.py)
- **Simplicity First**: Reproduces GPT-2 (124M) in ~600 lines
- **Minimal Dependencies**: Only torch, numpy, transformers, datasets
- **Education + Production**: "teeth over education" — practical but readable

### micrograd (~100 lines engine + ~50 lines nn)
- **Scalar-First**: Understand backprop at the atomic level (single value gradients)
- **First Principles**: Builds autograd from scratch, no PyTorch dependency for core
- **Clear Documentation**: README with working examples, visualizations

### build-nanogpt (step-by-step commits)
- **Reproduce from Scratch**: Starts from empty file, each commit is a clean step
- **Education over Production**: Git history IS the tutorial
- **Goal-Driven**: Each commit has a verifiable outcome

### llama2.c (pure C inference)
- **Minimal Dependencies**: Zero external deps. Pure C.
- **Simplicity First**: ~500 lines of C for full Llama 2 inference
- **First Principles**: No framework, just math and memory

---

## Swarm Application

### For Agents (Development Rules)

**When building features:**
1. **First Principles** — Break the feature to atomic units. Don't copy existing code blindly.
2. **Occam's Razor** — Minimum code that solves the problem. 50 lines > 200 lines.
3. **Inversion** — Before coding, define what would make this fail. Write that test first.
4. **Falsifiability** — Every claim ("this is faster", "this is more secure") needs a test.

**When reviewing code:**
1. **The Map is not the Territory** — The PR description is not the code. Read the actual diff.
2. **Circle of Competence** — If you don't understand a change, ask. Don't rubber-stamp.
3. **Hanlon's Razor** — Bugs are usually oversight, not malice. Be kind in review.

**When debugging:**
1. **Causation vs. Correlation** — The error might not be where it manifests. Trace actual causality.
2. **Second-Order Thinking** — Fixing this bug might create another. Think downstream.
3. **Probabilistic Thinking** — Multiple causes possible. Test each hypothesis.

---

## Connected

- [[wiki/concepts/First Principles Thinking]] — Breaking problems to fundamentals
- [[wiki/concepts/How to Read a Book]] — Adler's framework for learning from sources
- [[wiki/entities/Andrej Karpathy]] — Karpathy bio, repos, principles
- [[wiki/entities/Shane Parrish]] — Farnam Street, mental models curation
- [[wiki/syntheses/Syntopical Synthesis — Agentic AI + Knowledge Graphs]] — Cross-book analysis methodology
- [[wiki/concepts/Occam's Razor]] — Simplicity in system design
- [[AGENTS.md]] — Multi-agent protocol with these rules embedded
- [[CLAUDE.md]] — Claude-specific extensions with Karpathy guidelines

---

*Sources: Shane Parrish — Great Mental Models Vol 1 (227 pages), Andrej Karpathy — GitHub repos (nanoGPT, micrograd, build-nanogpt, llama2.c, minGPT), Karpathy LLM Coding Guidelines (2026)*
