# StyleFreeze

> **Stop paying Opus to reformat your files.**  
> AST-level surgical diff enforcer for Claude Code, Codex, and Cursor.

```
======================================================================
                    STYLEFREEZE AUDIT RECEIPT
             "Stop paying Opus to reformat your files"
======================================================================
 Repository:           my-saas-core
 Commits Analyzed:     50 commits
 Files Inspected:      142 source files
----------------------------------------------------------------------
 Total Diff Lines:     16,420
 Pure AST Churn:        9,840  [ 59.9% of total diffs! ]
 Contaminated Blames:      38 files
----------------------------------------------------------------------
 💸 Wasted Tokens:     137,760 Tokens
 💰 Estimated Burned:  $10.33 USD
======================================================================
 ⚠️  VERDICT: Your coding agent is burning ~60% of your token budget
            re-indenting and reformatting untouched functions.
======================================================================
 👉 Install shield: stylefreeze hook install claude
```

---

## The Problem

You asked your coding agent to fix a bug in line 42.

It modified the bug, but it also:
1. Re-indented the entire 400-line file from 4 spaces to 2 spaces.
2. Swapped all single quotes to double quotes.
3. Deleted inline comments in functions it was never instructed to touch.
4. Blew up your `git blame` history.
5. Burned 30,000 tokens sending the bloated diff back and forth in subsequent context turns.

Asking the model nicely in prose (`CLAUDE.md`, system prompts) fails ~10% of the time. LLMs have formatting preferences baked into their weights.

**StyleFreeze solves this deterministically at the AST layer.**

---

## How It Works

StyleFreeze sits between the LLM's file write call and your disk.

```
 [AI Agent Edit Action]
           │
           ▼
 ┌──────────────────┐
 │  StyleFreeze AST │ ── tree-sitter parses original vs. candidate
 └──────────────────┘
           │
     ┌─────┴────────────────────────┐
     ▼                              ▼
[Semantic Changes]         [Formatting & Quote Churn]
     │                              │
     ▼                              ▼
 Preserved & Applied        Reverted to Original Syntax
           │                              │
           └──────────────┬───────────────┘
                          │
                          ▼
            [Surgical Git Diff Written]
```

1. **Hunk AST Filtering**: Tree-sitter parses the abstract syntax tree of both versions. If an entire hunk or function declaration has an identical normalized AST structure, the entire hunk is rejected and reverted to the original.
2. **Intra-Hunk Line Normalization**: For lines with real logic edits, StyleFreeze isolates the exact sub-statement change while locking in the original line's indentation and quote conventions.
3. **Zero Token & Sub-5ms Overhead**: Pure Rust. No LLM judge. No cloud API calls. Executes in under 5 milliseconds.

---

## Quick Start

### 1. Audit your repo's wasted tokens (Read-Only)

See how much money and token context your coding agents have burned on formatting churn across your recent commits:

```bash
npx stylefreeze audit-git
```

Or via Cargo:

```bash
cargo install stylefreeze
stylefreeze audit-git --depth 50
```

### 2. Protect Claude Code

Automatically sanitize every `Edit` or `Write` tool call before it touches disk:

```bash
stylefreeze hook install claude
```

This writes a `PreToolUse` hook to `.claude/hooks.json`.

### 3. Protect via Git pre-commit

```bash
stylefreeze hook install git
```

### 4. CLI / Pipeline Usage

```bash
# Filter a file against git HEAD in-place
stylefreeze filter src/services/auth.ts --in-place

# Filter candidate against original
stylefreeze filter original.py --new-file candidate.py --in-place

# Pipe via stdin
cat candidate.ts | stylefreeze filter > surgical.ts
```

---

## Benchmark

Tested on a 1,200-line TypeScript file where Claude Opus modified 2 lines of logic and reformatted the rest:

| Metric | Without StyleFreeze | With StyleFreeze | Difference |
| :--- | :--- | :--- | :--- |
| **Lines Changed in Diff** | 384 lines | **2 lines** | **99.5% reduction** |
| **Git Blame Contamination** | 18 functions | **0 functions** | Clean history |
| **Subsequent Context Re-read** | ~8,400 tokens | **~450 tokens** | **94.6% saved** |
| **Execution Latency** | — | **3.8 ms** | Imperceptible |

---

## Supported Languages

- **TypeScript / TSX** (`.ts`, `.tsx`, `.mts`, `.cts`)
- **JavaScript / JSX** (`.js`, `.jsx`, `.mjs`, `.cjs`)
- **Python** (`.py`, `.pyi`)

*(More grammars arriving via tree-sitter plug-ins)*

---

## License

MIT
