use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use stylefreeze::audit::{print_receipt, run_git_audit};
use stylefreeze::engine::SurgicalEngine;
use stylefreeze::hook::{install_claude_hook, install_git_hook};
use stylefreeze::lang::SupportedLang;

#[derive(Parser)]
#[command(
    name = "stylefreeze",
    author = "StyleFreeze Contributors",
    version = "0.1.0",
    about = "AST-level surgical diff enforcer & token churn shield for AI coding agents",
    long_about = "StyleFreeze intercepts AI file modifications and strips unsolicited formatting, \
                  indentation, and cosmetic churn at the AST level, ensuring minimal diffs and \
                  preventing token waste."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Filter a modified file or stdin, reverting non-semantic AST churn
    Filter {
        /// Target file path
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,

        /// Path to the candidate/new file (if separate from target file)
        #[arg(long, short = 'n', value_name = "CANDIDATE")]
        new_file: Option<PathBuf>,

        /// Update the file in-place with surgical diff
        #[arg(long, short = 'i')]
        in_place: bool,

        /// Suppress terminal feedback messages
        #[arg(long, short = 'q')]
        quiet: bool,
    },

    /// Scan git history to audit tokens and money burned by AI formatting churn
    AuditGit {
        /// Number of recent commits to analyze (default: 50)
        #[arg(long, short = 'd', default_value_t = 50)]
        depth: usize,

        /// Target repository directory (default: current directory)
        #[arg(long, short = 'r', default_value = ".")]
        repo: PathBuf,
    },

    /// Install automatic surgical hooks for coding agents or git
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },
}

#[derive(Subcommand)]
enum HookAction {
    /// Install a hook
    Install {
        /// Target environment (claude | git)
        #[arg(value_name = "TARGET")]
        target: String,

        /// Target repository directory (default: current directory)
        #[arg(long, short = 'r', default_value = ".")]
        repo: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Filter {
            file,
            new_file,
            in_place,
            quiet,
        } => handle_filter(file, new_file, in_place, quiet),
        Commands::AuditGit { depth, repo } => handle_audit_git(&repo, depth),
        Commands::Hook { action } => match action {
            HookAction::Install { target, repo } => handle_hook_install(&target, &repo),
        },
    }
}

fn handle_filter(
    file_opt: Option<PathBuf>,
    new_file_opt: Option<PathBuf>,
    in_place: bool,
    quiet: bool,
) -> Result<()> {
    let (original_text, candidate_text, file_path, lang) = match (file_opt, new_file_opt) {
        // Case 1: Both original file and candidate file specified
        (Some(orig_path), Some(cand_path)) => {
            let orig = fs::read_to_string(&orig_path)
                .with_context(|| format!("Failed to read original file: {}", orig_path.display()))?;
            let cand = fs::read_to_string(&cand_path)
                .with_context(|| format!("Failed to read candidate file: {}", cand_path.display()))?;
            let lang = SupportedLang::from_path(&orig_path)
                .or_else(|| SupportedLang::from_path(&cand_path))
                .unwrap_or(SupportedLang::TypeScript);
            (orig, cand, Some(orig_path), lang)
        }
        // Case 2: In-place filtering on a file already modified on disk
        (Some(path), None) => {
            let candidate = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read modified file: {}", path.display()))?;
            let lang = SupportedLang::from_path(&path).unwrap_or(SupportedLang::TypeScript);

            // Fetch original version from git HEAD
            let relative_path = path.to_string_lossy();
            let git_show = Command::new("git")
                .args(["show", &format!("HEAD:{}", relative_path)])
                .output();

            let original = match git_show {
                Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
                _ => {
                    // Not tracked in git or newly added, nothing to revert against
                    return Ok(());
                }
            };

            (original, candidate, Some(path), lang)
        }
        // Case 3: Read from stdin
        (None, _) => {
            let mut stdin_buf = String::new();
            io::stdin().read_to_string(&mut stdin_buf)?;
            (stdin_buf.clone(), stdin_buf, None, SupportedLang::TypeScript)
        }
    };

    let engine = SurgicalEngine::new(lang);
    let (reconciled, stats) = engine.reconcile(&original_text, &candidate_text);

    if in_place {
        if let Some(ref path) = file_path {
            fs::write(path, &reconciled)
                .with_context(|| format!("Failed to write reconciled file: {}", path.display()))?;
        }
    } else if file_path.is_none() {
        io::stdout().write_all(reconciled.as_bytes())?;
    }

    if !quiet && stats.discarded_churn_lines > 0 {
        eprintln!(
            "{} Discarded {} lines of unsolicited format churn (Saved ~{} tokens).",
            "[StyleFreeze]".bright_cyan().bold(),
            stats.discarded_churn_lines.to_string().bright_yellow().bold(),
            stats.estimated_tokens_saved.to_string().bright_green().bold()
        );
    }

    Ok(())
}

fn handle_audit_git(repo: &Path, depth: usize) -> Result<()> {
    let abs_repo = fs::canonicalize(repo).unwrap_or_else(|_| repo.to_path_buf());
    let repo_name = abs_repo
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "current-repo".to_string());

    eprintln!(
        "{} Scanning past {} commits in {}...",
        "[StyleFreeze]".bright_cyan().bold(),
        depth,
        repo_name.bright_yellow()
    );

    let report = run_git_audit(&abs_repo, depth)?;
    print_receipt(&report, &repo_name);

    Ok(())
}

fn handle_hook_install(target: &str, repo: &Path) -> Result<()> {
    match target.to_lowercase().as_str() {
        "git" => install_git_hook(repo),
        "claude" | "claudecode" => install_claude_hook(repo),
        other => {
            anyhow::bail!(
                "Unsupported hook target: '{}'. Valid targets are: 'claude', 'git'",
                other
            );
        }
    }
}
