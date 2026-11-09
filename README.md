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
 [WASTE] Wasted Tokens:     137,760 Tokens
 [SAVED] Estimated Burned:  $10.33 USD
======================================================================
 [WARN] VERDICT: Your coding agent is burning ~60% of your token budget
                 re-indenting and reformatting untouched functions.
======================================================================
 [SHIELD] Install shield: stylefreeze hook install claude
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
2. **Comment & Trivia Shield**: If an AI lazily drops your existing comments without being asked, StyleFreeze detects the loss and automatically preserves your original inline documentation.
3. **Intra-Hunk Line Normalization**: For lines with real logic edits, StyleFreeze isolates the exact sub-statement change while locking in the original line's indentation (spaces or tabs) and quote conventions.
4. **Zero Token & Sub-3ms Overhead**: Pure Rust. No LLM judge. No cloud API calls. Executes in 2.5 milliseconds.

---

## Quick Start

### 1. Audit your repo's wasted tokens (Read-Only)

See how much money and token context your coding agents have burned on formatting churn across your recent commits:

```bash
npx stylefreeze audit-git
```

Or generate a clean Markdown table for your GitHub Pull Request comments:

```bash
stylefreeze audit-git --pr
```

### 2. Protect Claude Code

Automatically sanitize every `Edit` or `Write` tool call before it touches disk:

```bash
stylefreeze hook install claude
```

This writes a `PreToolUse` hook to `.claude/hooks.json`.

### 3. Protect Cursor

Configure Cursor rules and connect StyleFreeze as an MCP server:

```bash
stylefreeze hook install cursor
```

### 4. Protect via Git pre-commit

```bash
stylefreeze hook install git
```

### 5. CLI & Visual Diff Breakdown

```bash
# Filter a file against git HEAD in-place and see the visual breakdown
stylefreeze filter src/services/auth.ts --in-place --diff

# Filter candidate against original
stylefreeze filter original.py --new-file candidate.py --in-place

# Pipe via stdin
cat candidate.ts | stylefreeze filter > surgical.ts
```

### 6. Model Context Protocol (MCP) Server

StyleFreeze can run as a standard stdio MCP tool server:

```bash
stylefreeze mcp
```

Exposes `stylefreeze_reconcile` for autonomous multi-agent tool loops.

---

## Benchmark

Benchmarked on varying TypeScript codebases with simulated AI cosmetic reformatting + 1 real logic change:

| File Scale | Diff Reduction | StyleFreeze Latency |
| :--- | :--- | :--- |
| **300 lines** | **98.7%** | **2.53 ms** |
| **1,200 lines** | **99.5%** | **12.32 ms** |
| **3,000 lines** | **99.8%** | **44.34 ms** |

*Measured on Rust `1.99.1` with native `tree-sitter` AST traversal.*

---

## Supported Languages

- **TypeScript / TSX** (`.ts`, `.tsx`, `.mts`, `.cts`)
- **JavaScript / JSX** (`.js`, `.jsx`, `.mjs`, `.cjs`)
- **Python** (`.py`, `.pyi`)
- **Rust** (`.rs`)
- **Go** (`.go`)

---

## License

MIT
