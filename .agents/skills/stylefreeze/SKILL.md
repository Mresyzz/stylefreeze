---
name: stylefreeze
description: AST-level surgical diff enforcer. Use to audit or strip non-semantic formatting churn and token waste caused by AI coding agents.
---

# StyleFreeze Skill

StyleFreeze strips unsolicited formatting, quote rewrites, and indentation churn caused by LLM over-editing at the AST level.

## Supported Languages
- TypeScript & TSX
- JavaScript & JSX
- Python
- Rust
- Go
- JSON & JSONC

## When to Use

1. **One-click setup**: Run `stylefreeze init` to generate `stylefreeze.toml` and configure Claude Code, Cursor, and Git hooks simultaneously.
2. **CI gate & verify**: Run `stylefreeze check` to audit working tree or staged files, or `stylefreeze check --fix` to auto-fix.
3. **Audit token waste**: Run `stylefreeze audit-git --depth 50` to inspect past commits for formatting churn, or `stylefreeze audit-git --pr` for PR summaries.
4. **Surgical editing**: After generating a candidate edit on source files, run `stylefreeze filter <file> --in-place` to discard whitespace/formatting noise before committing or re-reading into context.
5. **Inspect changes**: Pass `--diff` (`stylefreeze filter <file> --in-place --diff`) to see what was frozen vs applied.
6. **Automatic hook**: Run `stylefreeze hook install claude`, `stylefreeze hook install cursor`, or `stylefreeze hook install git`.
7. **MCP Server**: Launch with `stylefreeze mcp` to connect via stdio to any MCP client.
