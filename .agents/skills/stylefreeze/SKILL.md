---
name: stylefreeze
description: AST-level surgical diff enforcer. Use to audit or strip non-semantic formatting churn and token waste caused by AI coding agents.
---

# StyleFreeze Skill

StyleFreeze strips unsolicited formatting, quote rewrites, and indentation churn caused by LLM over-editing at the AST level.

## When to Use

1. **Audit token waste**: Run `stylefreeze audit-git --depth 50` to inspect past commits for formatting churn.
2. **Surgical editing**: After generating a candidate edit on TypeScript, JavaScript, or Python files, run `stylefreeze filter <file> --in-place` to discard whitespace/formatting noise before committing or re-reading into context.
3. **Automatic hook**: Run `stylefreeze hook install claude` or `stylefreeze hook install git` to enforce surgical diffs automatically.

## Quick CLI Reference

```bash
# Scan git history and print the waste receipt
stylefreeze audit-git

# Filter a file against its git HEAD in-place
stylefreeze filter path/to/file.ts --in-place

# Filter candidate against original
stylefreeze filter original.py --new-file candidate.py --in-place
```
