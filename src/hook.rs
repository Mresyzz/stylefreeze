use colored::*;
use std::fs;
use std::path::Path;

pub fn install_git_hook(repo_dir: &Path) -> anyhow::Result<()> {
    let hooks_dir = repo_dir.join(".git").join("hooks");
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir)?;
    }

    let pre_commit_path = hooks_dir.join("pre-commit");

    let script = r#"#!/usr/bin/env sh
# StyleFreeze pre-commit surgical hook
# Rejects or reconciles non-semantic AST formatting churn

if command -v stylefreeze >/dev/null 2>&1; then
    # Get staged files
    STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(ts|tsx|js|jsx|py|rs|go)$')
    for FILE in $STAGED_FILES; do
        if [ -f "$FILE" ]; then
            # Filter in-place
            stylefreeze filter "$FILE" --in-place >/dev/null 2>&1
            git add "$FILE"
        fi
    done
fi
exit 0
"#;

    fs::write(&pre_commit_path, script)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&pre_commit_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&pre_commit_path, perms)?;
    }

    println!(
        "{} Git pre-commit hook installed to {}",
        "✔".bright_green().bold(),
        pre_commit_path.display()
    );
    Ok(())
}

pub fn install_claude_hook(repo_dir: &Path) -> anyhow::Result<()> {
    let claude_dir = repo_dir.join(".claude");
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)?;
    }

    let hooks_config = claude_dir.join("hooks.json");
    let content = serde_json::json!({
        "version": "1.0",
        "description": "StyleFreeze AST Surgical Shield for Claude Code",
        "hooks": {
            "PreToolUse": [
                {
                    "matcher": "Edit|MultiEdit|Write",
                    "command": "stylefreeze filter --in-place \"$TOOL_INPUT_FILE\""
                }
            ]
        }
    });

    fs::write(&hooks_config, serde_json::to_string_pretty(&content)?)?;

    println!(
        "{} Claude Code surgical hook configured in {}",
        "✔".bright_green().bold(),
        hooks_config.display()
    );
    Ok(())
}

pub fn install_cursor_hook(repo_dir: &Path) -> anyhow::Result<()> {
    let cursor_dir = repo_dir.join(".cursor");
    let rules_dir = cursor_dir.join("rules");
    if !rules_dir.exists() {
        fs::create_dir_all(&rules_dir)?;
    }

    // 1. Cursor MDC rule
    let rule_file = rules_dir.join("stylefreeze.mdc");
    let rule_content = r#"---
description: StyleFreeze Surgical Diff Rules
globs: *
alwaysApply: true
---

# StyleFreeze AST Shield

When modifying files in this repository:
1. Make surgical edits only. Touch the exact lines required for the task.
2. Do not reformat, re-indent, or rewrite untouched functions.
3. Do not alter existing quote styles or remove existing code comments.
4. StyleFreeze AST filter runs before commits and verifies zero formatting churn.
"#;
    fs::write(&rule_file, rule_content)?;

    // 2. Cursor MCP configuration
    let mcp_file = cursor_dir.join("mcp.json");
    let mcp_config = serde_json::json!({
        "mcpServers": {
            "stylefreeze": {
                "command": "stylefreeze",
                "args": ["mcp"]
            }
        }
    });
    fs::write(&mcp_file, serde_json::to_string_pretty(&mcp_config)?)?;

    println!(
        "{} Cursor MDC rule and MCP server configured in {}",
        "✔".bright_green().bold(),
        cursor_dir.display()
    );
    Ok(())
}
