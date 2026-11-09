# StyleFreeze

> **Stop paying Opus to reformat your files.**  
> AST-level surgical diff enforcer for Claude Code, Codex, and Cursor.

[![CI](https://github.com/Mresyzz/stylefreeze/actions/workflows/ci.yml/badge.svg)](https://github.com/Mresyzz/stylefreeze/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Mresyzz/stylefreeze?color=blue)](https://github.com/Mresyzz/stylefreeze/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Latency: Sub-3ms](https://img.shields.io/badge/Latency-%3C3ms-brightgreen.svg)]()
[![Languages](https://img.shields.io/badge/Languages-TS%20%7C%20JS%20%7C%20Python%20%7C%20Rust%20%7C%20Go%20%7C%20C%2F%2B%2B%20%7C%20JSON-orange.svg)]()

```text
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

```text
 [AI Agent Edit Action]
           |
           v
 +------------------+
 |  StyleFreeze AST | -- tree-sitter parses original vs. candidate
 +------------------+
           |
     +-----+------------------------+
     v                              v
[Semantic Changes]         [Formatting & Quote Churn]
     |                              |
     v                              v
 Preserved & Applied        Reverted to Original Syntax
           |                              |
           +--------------+---------------+
                          |
                          v
            [Surgical Git Diff Written]
```

1. **Hunk AST Filtering**: Tree-sitter parses the abstract syntax tree of both versions. If an entire hunk or function declaration has an identical normalized AST structure, the entire hunk is rejected and reverted to the original.
2. **Comment & Trivia Shield**: If an AI lazily drops your existing comments without being asked, StyleFreeze detects the loss and automatically preserves your original inline documentation.
3. **Intra-Hunk Line Normalization**: For lines with real logic edits, StyleFreeze isolates the exact sub-statement change while locking in the original line's indentation (spaces or tabs) and quote conventions.
4. **Zero Token & Sub-3ms Overhead**: Pure Rust. No LLM judge. No cloud API calls. Executes in 2.5 milliseconds.

---

## Installation

### One-Line Install (Prebuilt Binaries)

**Linux & macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/Mresyzz/stylefreeze/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/Mresyzz/stylefreeze/main/install.ps1 | iex
```

### Via Cargo
```bash
cargo install stylefreeze
```

### Via NPM / NPX
```bash
npx stylefreeze audit-git
```

---

## Quick Start

### 1. One-Click Initialization (Recommended)

Run `stylefreeze init` in any repository to automatically generate `stylefreeze.toml` and configure hooks for all detected agents (Claude Code, Cursor, and Git pre-commit):

```bash
stylefreeze init
```

### 2. CI Gate & Working Tree Check

Inspect modified files in the working tree or staged changes, and automatically sanitize them:

```bash
# Check working tree (exits with code 1 if unsolicited churn is found)
stylefreeze check

# Check staged files only
stylefreeze check --staged

# Automatically reconcile and fix files in-place
stylefreeze check --fix
```

### 3. Audit your repo's wasted tokens (Read-Only)

See how much money and token context your coding agents have burned on formatting churn across your recent commits, or check staged changes before committing:

```bash
# Audit past commits
stylefreeze audit-git

# Audit currently staged changes before commit
stylefreeze audit-git --staged

# Generate a clean Markdown table for Pull Request comments
stylefreeze audit-git --pr
```

### 4. Protect Claude Code

Automatically sanitize every `Edit` or `Write` tool call before it touches disk:

```bash
stylefreeze hook install claude
```

This writes a `PreToolUse` hook to `.claude/hooks.json`.

### 5. Protect Cursor

Configure Cursor rules and connect StyleFreeze as an MCP server:

```bash
stylefreeze hook install cursor
```

### 6. Protect via Git pre-commit

```bash
stylefreeze hook install git
```

### 7. Interactive Visual Diff

Inspect what unsolicited churn would be reverted and what real logic would be kept:

```bash
# Diff working tree changes for all modified source files
stylefreeze diff

# Diff git staged changes only
stylefreeze diff --staged

# Diff a single file against git HEAD
stylefreeze diff src/services/auth.ts
```

### 8. Import Shield (Order & Blank Line Protection)

AI agents frequently re-alphabetize or re-order imports when adding a single package. StyleFreeze's Import Shield automatically freezes original import grouping and order, appending only genuine new imports to avoid 20+ lines of red/green diff churn across TypeScript, Python, Rust, Go, and C/C++.

### 9. CLI Surgical Filtering

```bash
# Filter a file against git HEAD in-place and see the visual breakdown
stylefreeze filter src/services/auth.ts --in-place --diff

# Filter candidate against original
stylefreeze filter original.py --new-file candidate.py --in-place

# Pipe via stdin
cat candidate.ts | stylefreeze filter > surgical.ts
```

### 10. Model Context Protocol (MCP) Server

StyleFreeze can run as a standard stdio MCP tool server:

```bash
stylefreeze mcp
```

Exposes `stylefreeze_reconcile` for autonomous multi-agent tool loops in Windsurf, Cursor, Zed, and Claude Desktop.

---

## GitHub Actions Integration

Add StyleFreeze as a Pull Request audit shield to automatically post token waste savings on incoming PRs:

```yaml
name: StyleFreeze Audit

on:
  pull_request:

jobs:
  audit:
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: StyleFreeze PR Shield
        uses: Mresyzz/stylefreeze@v0.2.0
```

---

## Benchmark

Benchmarked on varying TypeScript codebases with simulated AI cosmetic reformatting + 1 real logic change:

| File Scale | Diff Reduction | StyleFreeze Latency |
| :--- | :--- | :--- |
| **300 lines** | **98.7%** | **2.53 ms** |
| **1,200 lines** | **99.5%** | **12.32 ms** |
| **3,000 lines** | **99.8%** | **44.34 ms** |

*Measured on Rust 1.85+ with native `tree-sitter` AST traversal.*

---

## Supported Languages

- **TypeScript / TSX** (`.ts`, `.tsx`, `.mts`, `.cts`)
- **JavaScript / JSX** (`.js`, `.jsx`, `.mjs`, `.cjs`)
- **Python** (`.py`, `.pyi`)
- **Rust** (`.rs`)
- **Go** (`.go`)
- **C** (`.c`, `.h`)
- **C++** (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh`, `.hxx`)
- **JSON / JSONC** (`.json`, `.jsonc`)

---

## License

MIT
