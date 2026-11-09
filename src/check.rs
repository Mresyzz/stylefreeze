use crate::engine::SurgicalEngine;
use crate::lang::SupportedLang;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
pub struct CheckResult {
    pub file_path: PathBuf,
    pub churn_lines: usize,
    pub tokens_wasted: usize,
    pub fixed: bool,
}

pub fn run_check(
    repo_dir: &Path,
    staged_only: bool,
    fix: bool,
) -> anyhow::Result<(Vec<CheckResult>, bool)> {
    let git_args = if staged_only {
        vec!["diff", "--cached", "--name-only", "--diff-filter=ACM"]
    } else {
        vec!["diff", "HEAD", "--name-only", "--diff-filter=ACM"]
    };

    let output = Command::new("git")
        .args(&git_args)
        .current_dir(repo_dir)
        .output()?;

    let file_list = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();
    let mut all_clean = true;

    for rel_path in file_list.lines() {
        let rel_path = rel_path.trim();
        if rel_path.is_empty() {
            continue;
        }

        let full_path = repo_dir.join(rel_path);
        let lang = match SupportedLang::from_path(&full_path) {
            Some(l) => l,
            None => continue,
        };

        if !full_path.exists() {
            continue;
        }

        let candidate_text = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let git_show = Command::new("git")
            .args(["show", &format!("HEAD:{}", rel_path)])
            .current_dir(repo_dir)
            .output();

        let original_text = match git_show {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
            _ => continue,
        };

        let engine = SurgicalEngine::new(lang);
        let (reconciled, stats) = engine.reconcile(&original_text, &candidate_text);

        if stats.discarded_churn_lines > 0 {
            all_clean = false;
            let mut is_fixed = false;

            if fix && fs::write(&full_path, &reconciled).is_ok() {
                is_fixed = true;
                if staged_only {
                    let _ = Command::new("git")
                        .args(["add", rel_path])
                        .current_dir(repo_dir)
                        .output();
                }
            }

            results.push(CheckResult {
                file_path: full_path,
                churn_lines: stats.discarded_churn_lines,
                tokens_wasted: stats.estimated_tokens_saved,
                fixed: is_fixed,
            });
        }
    }

    Ok((results, all_clean))
}
