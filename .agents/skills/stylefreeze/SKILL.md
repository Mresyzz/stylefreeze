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

## When to Use

1. **Audit token waste**: Run `stylefreeze audit-git --depth 50` to inspect past commits for formatting churn, or `stylefreeze audit-git --pr` for PR summaries.
2. **Surgical editing**: After generating a candidate edit on source files, run `stylefreeze filter <file> --in-place` to discard whitespace/formatting noise before committing or re-reading into context.
3. **Inspect changes**: Pass `--diff` (`stylefreeze filter <file> --in-place --diff`) to see what was frozen vs applied.
4. **Automatic hook**: Run `stylefreeze hook install claude`, `stylefreeze hook install cursor`, or `stylefreeze hook install git`.
5. **MCP Server**: Launch with `stylefreeze mcp` to connect via stdio to any MCP client.
