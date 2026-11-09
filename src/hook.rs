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
    STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(ts|tsx|js|jsx|py)$')
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
