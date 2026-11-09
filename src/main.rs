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
    version = "0.4.0",
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
    /// Initialize StyleFreeze configuration and install hooks for all detected agents
    Init {
        /// Target repository directory (default: current directory)
        #[arg(long, short = 'r', default_value = ".")]
        repo: PathBuf,
    },

    /// Check working tree or staged files for unsolicited formatting churn
    Check {
        /// Target repository directory (default: current directory)
        #[arg(long, short = 'r', default_value = ".")]
        repo: PathBuf,

        /// Only inspect git staged files
        #[arg(long, short = 's')]
        staged: bool,

        /// Automatically fix and reconcile files in-place
        #[arg(long, short = 'f')]
        fix: bool,
    },

    /// Display visual surgical breakdown of churn vs applied logic for modified or staged files
    Diff {
        /// Target file to inspect
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,

        /// Path to separate candidate file
        #[arg(long, short = 'n', value_name = "CANDIDATE")]
        new_file: Option<PathBuf>,

        /// Inspect git staged files
        #[arg(long, short = 's')]
        staged: bool,

        /// Target repository directory (default: current directory)
        #[arg(long, short = 'r', default_value = ".")]
        repo: PathBuf,
    },

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

        /// Display visual breakdown of reverted churn vs applied changes
        #[arg(long, short = 'd')]
        diff: bool,
    },

    /// Scan git history to audit tokens and money burned by AI formatting churn
    AuditGit {
        /// Number of recent commits to analyze (default: 50)
        #[arg(long, short = 'd', default_value_t = 50)]
        depth: usize,

        /// Target repository directory (default: current directory)
        #[arg(long, short = 'r', default_value = ".")]
        repo: PathBuf,

        /// Output report in JSON format
        #[arg(long)]
        json: bool,

        /// Output report in GitHub PR Markdown comment format
        #[arg(long)]
        pr: bool,

        /// Audit currently staged changes instead of past commit history
        #[arg(long, short = 's')]
        staged: bool,
    },

    /// Run as a Model Context Protocol (MCP) server over stdio
    Mcp,

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
        Commands::Init { repo } => handle_init(&repo),
        Commands::Check { repo, staged, fix } => handle_check(&repo, staged, fix),
        Commands::Diff {
            file,
            new_file,
            staged,
            repo,
        } => handle_diff(&repo, file, new_file, staged),
        Commands::Filter {
            file,
            new_file,
            in_place,
            quiet,
            diff,
        } => handle_filter(file, new_file, in_place, quiet, diff),
        Commands::AuditGit {
            depth,
            repo,
            json,
            pr,
            staged,
        } => handle_audit_git(&repo, depth, json, pr, staged),
        Commands::Mcp => stylefreeze::mcp::run_mcp_server(),
        Commands::Hook { action } => match action {
            HookAction::Install { target, repo } => handle_hook_install(&target, &repo),
        },
    }
}

fn handle_init(repo: &Path) -> Result<()> {
    let abs_repo = fs::canonicalize(repo).unwrap_or_else(|_| repo.to_path_buf());
    let config_path = abs_repo.join("stylefreeze.toml");

    println!(
        "{} Initializing StyleFreeze in {}...",
        "[StyleFreeze]".bright_cyan().bold(),
        abs_repo.display().to_string().bright_yellow()
    );

    if !config_path.exists() {
        stylefreeze::config::StyleFreezeConfig::write_default(&config_path)?;
        println!(
            " {} Created configuration file: {}",
            "[OK]".bright_green().bold(),
            "stylefreeze.toml".bold()
        );
    } else {
        println!(
            " {} Configuration file already exists: {}",
            "[INFO]".cyan().bold(),
            "stylefreeze.toml".bold()
        );
    }

    if abs_repo.join(".git").exists() {
        let _ = install_git_hook(&abs_repo);
    }
    let _ = install_claude_hook(&abs_repo);
    let _ = stylefreeze::hook::install_cursor_hook(&abs_repo);

    println!(
        "\n{} StyleFreeze successfully initialized. Your repo is now protected.",
        "[OK]".bright_green().bold()
    );

    Ok(())
}

fn handle_check(repo: &Path, staged: bool, fix: bool) -> Result<()> {
    let abs_repo = fs::canonicalize(repo).unwrap_or_else(|_| repo.to_path_buf());
    let (results, all_clean) = stylefreeze::check::run_check(&abs_repo, staged, fix)?;

    if all_clean {
        println!(
            "{} All inspected files are surgically clean. Zero formatting churn detected.",
            "[OK]".bright_green().bold()
        );
        return Ok(());
    }

    if fix {
        for res in &results {
            println!(
                " {} Fixed {} (Stripped {} lines of churn, saved ~{} tokens)",
                "[OK]".bright_green().bold(),
                res.file_path.display().to_string().bold(),
                res.churn_lines.to_string().bright_yellow().bold(),
                res.tokens_wasted.to_string().bright_green().bold()
            );
        }
        println!(
            "\n{} Successfully fixed {} file(s).",
            "[OK]".bright_green().bold(),
            results.len()
        );
        Ok(())
    } else {
        for res in &results {
            println!(
                " {} {}: {} lines of unsolicited AST churn detected (Wasted ~{} tokens)",
                "[FAIL]".bright_red().bold(),
                res.file_path.display().to_string().bold(),
                res.churn_lines.to_string().bright_yellow().bold(),
                res.tokens_wasted.to_string().bright_red().bold()
            );
        }
        eprintln!(
            "\n{} Run `stylefreeze check --fix` to automatically sanitize these files.",
            "[TIP]".cyan().bold()
        );
        std::process::exit(1);
    }
}

fn handle_diff(
    repo: &Path,
    file_opt: Option<PathBuf>,
    new_file_opt: Option<PathBuf>,
    staged: bool,
) -> Result<()> {
    let abs_repo = fs::canonicalize(repo).unwrap_or_else(|_| repo.to_path_buf());

    // Case 1: Two explicit files provided (original and candidate)
    if let (Some(orig_path), Some(cand_path)) = (&file_opt, &new_file_opt) {
        let orig = fs::read_to_string(orig_path)
            .with_context(|| format!("Failed to read original file: {}", orig_path.display()))?;
        let cand = fs::read_to_string(cand_path)
            .with_context(|| format!("Failed to read candidate file: {}", cand_path.display()))?;
        let lang = SupportedLang::from_path(orig_path)
            .or_else(|| SupportedLang::from_path(cand_path))
            .unwrap_or(SupportedLang::TypeScript);

        let engine = SurgicalEngine::new(lang);
        let (reconciled, _) = engine.reconcile(&orig, &cand);
        print!("{}", engine.render_visual_diff(&orig, &cand, &reconciled));
        return Ok(());
    }

    // Case 2: Single file provided (compare git HEAD with working tree / staged)
    if let Some(target_file) = &file_opt {
        let abs_file = if target_file.is_absolute() {
            target_file.clone()
        } else {
            abs_repo.join(target_file)
        };

        let rel_path = abs_file.strip_prefix(&abs_repo).unwrap_or(&abs_file);
        let rel_posix = rel_path.to_string_lossy().replace('\\', "/");

        let output = Command::new("git")
            .args(["show", &format!("HEAD:{}", rel_posix)])
            .current_dir(&abs_repo)
            .output()
            .with_context(|| format!("Failed to read git HEAD for {}", rel_posix))?;

        if !output.status.success() {
            anyhow::bail!("File is not tracked in git HEAD: {}", rel_posix);
        }

        let orig = String::from_utf8_lossy(&output.stdout).to_string();
        let cand = if staged {
            let staged_output = Command::new("git")
                .args(["show", &format!(":{}", rel_posix)])
                .current_dir(&abs_repo)
                .output()?;
            String::from_utf8_lossy(&staged_output.stdout).to_string()
        } else {
            fs::read_to_string(&abs_file)?
        };

        let lang = SupportedLang::from_path(&abs_file).unwrap_or(SupportedLang::TypeScript);
        let engine = SurgicalEngine::new(lang);
        let (reconciled, _) = engine.reconcile(&orig, &cand);
        print!("{}", engine.render_visual_diff(&orig, &cand, &reconciled));
        return Ok(());
    }

    // Case 3: Repo-wide visual diff for modified or staged files
    let status_out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&abs_repo)
        .output()
        .context("Failed to run git status")?;

    let stdout_str = String::from_utf8_lossy(&status_out.stdout);
    let mut files_diffed = 0usize;

    for line in stdout_str.lines() {
        if line.len() < 3 {
            continue;
        }
        let index_status = line.as_bytes()[0];
        let worktree_status = line.as_bytes()[1];
        let path_str = line[3..].trim();

        let is_target = if staged {
            index_status != b' ' && index_status != b'?'
        } else {
            worktree_status == b'M' || index_status == b'M'
        };

        if !is_target {
            continue;
        }

        let file_path = abs_repo.join(path_str);
        let lang = match SupportedLang::from_path(&file_path) {
            Some(l) => l,
            None => continue,
        };

        let rel_posix = path_str.replace('\\', "/");
        let head_out = Command::new("git")
            .args(["show", &format!("HEAD:{}", rel_posix)])
            .current_dir(&abs_repo)
            .output();

        let orig = match head_out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => continue,
        };

        let cand = if staged {
            let s_out = Command::new("git")
                .args(["show", &format!(":{}", rel_posix)])
                .current_dir(&abs_repo)
                .output();
            match s_out {
                Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
                _ => continue,
            }
        } else {
            match fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => continue,
            }
        };

        if orig == cand {
            continue;
        }

        println!("\n{} {}", "File:".bold(), rel_posix.bright_yellow().bold());
        let engine = SurgicalEngine::new(lang);
        let (reconciled, _) = engine.reconcile(&orig, &cand);
        print!("{}", engine.render_visual_diff(&orig, &cand, &reconciled));
        files_diffed += 1;
    }

    if files_diffed == 0 {
        println!(
            "{} No modified files with formatting churn detected.",
            "[OK]".bright_green().bold()
        );
    }

    Ok(())
}

fn handle_filter(
    file_opt: Option<PathBuf>,
    new_file_opt: Option<PathBuf>,
    in_place: bool,
    quiet: bool,
    show_diff: bool,
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

    if show_diff {
        eprintln!("{}", engine.render_visual_diff(&original_text, &candidate_text, &reconciled));
    } else if !quiet && stats.discarded_churn_lines > 0 {
        eprintln!(
            "{} Discarded {} lines of unsolicited format churn (Saved ~{} tokens).",
            "[StyleFreeze]".bright_cyan().bold(),
            stats.discarded_churn_lines.to_string().bright_yellow().bold(),
            stats.estimated_tokens_saved.to_string().bright_green().bold()
        );
    }

    Ok(())
}

fn handle_audit_git(
    repo: &Path,
    depth: usize,
    json_output: bool,
    pr_output: bool,
    staged: bool,
) -> Result<()> {
    let abs_repo = fs::canonicalize(repo).unwrap_or_else(|_| repo.to_path_buf());
    let repo_name = abs_repo
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "current-repo".to_string());

    if !json_output && !pr_output {
        if staged {
            eprintln!(
                "{} Auditing staged changes in {}...",
                "[StyleFreeze]".bright_cyan().bold(),
                repo_name.bright_yellow()
            );
        } else {
            eprintln!(
                "{} Scanning past {} commits in {}...",
                "[StyleFreeze]".bright_cyan().bold(),
                depth,
                repo_name.bright_yellow()
            );
        }
    }

    let report = if staged {
        stylefreeze::audit::run_git_audit_staged(&abs_repo)?
    } else {
        run_git_audit(&abs_repo, depth)?
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if pr_output {
        println!("{}", stylefreeze::audit::render_pr_markdown(&report));
    } else {
        print_receipt(&report, &repo_name);
    }

    Ok(())
}

fn handle_hook_install(target: &str, repo: &Path) -> Result<()> {
    match target.to_lowercase().as_str() {
        "git" => install_git_hook(repo),
        "claude" | "claudecode" => install_claude_hook(repo),
        "cursor" => stylefreeze::hook::install_cursor_hook(repo),
        other => {
            anyhow::bail!(
                "Unsupported hook target: '{}'. Valid targets are: 'claude', 'cursor', 'git'",
                other
            );
        }
    }
}
