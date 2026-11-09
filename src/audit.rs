use crate::engine::SurgicalEngine;
use crate::lang::SupportedLang;
use colored::*;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Default)]
pub struct AuditReport {
    pub commits_analyzed: usize,
    pub files_inspected: usize,
    pub total_raw_diff_lines: usize,
    pub total_churn_lines: usize,
    pub contaminated_files_count: usize,
    pub estimated_tokens_wasted: usize,
}

pub fn run_git_audit(repo_dir: &Path, depth: usize) -> anyhow::Result<AuditReport> {
    // Check if git is available
    let git_check = Command::new("git")
        .arg("--version")
        .current_dir(repo_dir)
        .output();

    if git_check.is_err() {
        anyhow::bail!("git command not found or not accessible in PATH");
    }

    // Get commit hashes
    let output = Command::new("git")
        .args(["log", &format!("-n{}", depth), "--pretty=format:%H"])
        .current_dir(repo_dir)
        .output()?;

    let commit_list = String::from_utf8_lossy(&output.stdout);
    let commits: Vec<&str> = commit_list.lines().filter(|s| !s.trim().is_empty()).collect();

    if commits.is_empty() {
        return Ok(AuditReport::default());
    }

    let mut report = AuditReport {
        commits_analyzed: commits.len(),
        ..Default::default()
    };

    let mut contaminated_files = std::collections::HashSet::new();

    for commit in &commits {
        // Get modified files in this commit
        let diff_tree = Command::new("git")
            .args(["diff-tree", "--no-commit-id", "--name-only", "-r", commit])
            .current_dir(repo_dir)
            .output()?;

        let files_output = String::from_utf8_lossy(&diff_tree.stdout);
        for file_path in files_output.lines() {
            let file_path = file_path.trim();
            if file_path.is_empty() {
                continue;
            }

            let path = Path::new(file_path);
            let lang = match SupportedLang::from_path(path) {
                Some(l) => l,
                None => continue,
            };

            report.files_inspected += 1;

            // Fetch parent version: commit^:file_path
            let parent_res = Command::new("git")
                .args(["show", &format!("{}^:{}", commit, file_path)])
                .current_dir(repo_dir)
                .output();

            let parent_text = match parent_res {
                Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
                _ => continue, // newly added file in this commit, skip
            };

            // Fetch current commit version: commit:file_path
            let commit_res = Command::new("git")
                .args(["show", &format!("{}:{}", commit, file_path)])
                .current_dir(repo_dir)
                .output()?;

            if !commit_res.status.success() {
                continue;
            }

            let commit_text = String::from_utf8_lossy(&commit_res.stdout).to_string();

            let engine = SurgicalEngine::new(lang);
            let (_reconciled, stats) = engine.reconcile(&parent_text, &commit_text);

            report.total_raw_diff_lines += stats.raw_added_lines + stats.raw_removed_lines;
            report.total_churn_lines += stats.discarded_churn_lines;
            report.estimated_tokens_wasted += stats.estimated_tokens_saved;

            if stats.discarded_churn_lines > 0 {
                contaminated_files.insert(file_path.to_string());
            }
        }
    }

    report.contaminated_files_count = contaminated_files.len();

    Ok(report)
}

pub fn print_receipt(report: &AuditReport, repo_name: &str) {
    let waste_percentage = if report.total_raw_diff_lines > 0 {
        (report.total_churn_lines as f64 / report.total_raw_diff_lines as f64) * 100.0
    } else {
        0.0
    };

    // Opus/Sonnet blended pricing: approx $0.000075 per token for prompt roundtrips
    let cost_usd = (report.estimated_tokens_wasted as f64) * 0.000075;

    println!();
    println!("{}", "======================================================================".bold());
    println!("{}", "                    STYLEFREEZE AUDIT RECEIPT".bold().cyan());
    println!("{}", "             \"Stop paying Opus to reformat your files\"".dimmed());
    println!("{}", "======================================================================".bold());
    println!(" Repository:           {}", repo_name.bright_yellow());
    println!(" Commits Analyzed:     {} commits", report.commits_analyzed);
    println!(" Files Inspected:      {} source files", report.files_inspected);
    println!("{}", "----------------------------------------------------------------------".dimmed());
    println!(" Total Diff Lines:     {}", report.total_raw_diff_lines);
    println!(
        " Pure AST Churn:       {} {}",
        report.total_churn_lines.to_string().bright_red().bold(),
        format!("[ {:.1}% of total diffs! ]", waste_percentage).bright_red()
    );
    println!(" Contaminated Blames:  {} files", report.contaminated_files_count);
    println!("{}", "----------------------------------------------------------------------".dimmed());
    println!(
        " 💸 Wasted Tokens:     {} Tokens",
        report.estimated_tokens_wasted.to_string().bright_yellow().bold()
    );
    println!(
        " 💰 Estimated Burned:  {}",
        format!("${:.2} USD", cost_usd).bright_green().bold()
    );
    println!("{}", "======================================================================".bold());
    if report.total_churn_lines > 0 {
        println!(
            " {}",
            "⚠️  VERDICT: Your coding agent is burning tokens reformatting untouched code."
                .yellow()
                .bold()
        );
    } else {
        println!(
            " {}",
            "✅ VERDICT: Clean repository! No unsolicited formatting churn detected."
                .green()
                .bold()
        );
    }
    println!("{}", "======================================================================".bold());
    println!(
        " 👉 Install shield: {}",
        "stylefreeze hook install claude".cyan().bold()
    );
    println!();
}
