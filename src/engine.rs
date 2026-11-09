use crate::ast::are_nodes_semantically_equivalent;
use crate::lang::SupportedLang;
use similar::{ChangeTag, DiffOp, TextDiff};
use std::collections::HashSet;
use tree_sitter::Parser;

#[derive(Debug, Clone, Default)]
pub struct SurgicalStats {
    pub original_lines: usize,
    pub candidate_lines: usize,
    pub raw_added_lines: usize,
    pub raw_removed_lines: usize,
    pub surgical_added_lines: usize,
    pub surgical_removed_lines: usize,
    pub discarded_churn_lines: usize,
    pub estimated_tokens_saved: usize,
}

pub struct SurgicalEngine {
    lang: SupportedLang,
}

impl SurgicalEngine {
    pub fn new(lang: SupportedLang) -> Self {
        Self { lang }
    }

    pub fn reconcile(&self, original: &str, candidate: &str) -> (String, SurgicalStats) {
        if original == candidate {
            let line_count = original.lines().count();
            return (
                original.to_string(),
                SurgicalStats {
                    original_lines: line_count,
                    candidate_lines: line_count,
                    ..Default::default()
                },
            );
        }

        let (candidate_buf, import_churn) =
            reconcile_import_header(self.lang, original, candidate);
        let candidate = candidate_buf.as_str();

        let mut parser = Parser::new();
        let ts_lang = self.lang.to_tree_sitter_language();
        if parser.set_language(&ts_lang).is_err() {
            return self.fallback_stats(original, candidate);
        }

        let orig_tree = match parser.parse(original, None) {
            Some(t) => t,
            None => return self.fallback_stats(original, candidate),
        };

        let cand_tree = match parser.parse(candidate, None) {
            Some(t) => t,
            None => return self.fallback_stats(original, candidate),
        };

        if cand_tree.root_node().has_error() {
            return self.fallback_stats(original, candidate);
        }

        let diff = TextDiff::from_lines(original, candidate);
        let mut reconciled_lines: Vec<String> = Vec::new();

        let mut raw_added = 0usize;
        let mut raw_removed = 0usize;
        let mut discarded_churn = import_churn;

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Insert => raw_added += 1,
                ChangeTag::Delete => raw_removed += 1,
                ChangeTag::Equal => {}
            }
        }

        for op in diff.ops() {
            match *op {
                DiffOp::Equal { old_index, len, .. } => {
                    let orig_slice = original.lines().skip(old_index).take(len);
                    for line in orig_slice {
                        reconciled_lines.push(line.to_string());
                    }
                }
                DiffOp::Delete {
                    old_index, old_len, ..
                } => {
                    let orig_slice: Vec<&str> =
                        original.lines().skip(old_index).take(old_len).collect();
                    let all_whitespace = orig_slice.iter().all(|l| l.trim().is_empty());
                    if all_whitespace {
                        for l in orig_slice {
                            reconciled_lines.push(l.to_string());
                            discarded_churn += 1;
                        }
                    }
                }
                DiffOp::Insert {
                    new_index, new_len, ..
                } => {
                    let cand_slice: Vec<&str> =
                        candidate.lines().skip(new_index).take(new_len).collect();
                    let all_whitespace = cand_slice.iter().all(|l| l.trim().is_empty());
                    if all_whitespace {
                        discarded_churn += new_len;
                    } else {
                        for l in cand_slice {
                            reconciled_lines.push(l.to_string());
                        }
                    }
                }
                DiffOp::Replace {
                    old_index,
                    old_len,
                    new_index,
                    new_len,
                } => {
                    let orig_slice: Vec<&str> =
                        original.lines().skip(old_index).take(old_len).collect();
                    let cand_slice: Vec<&str> =
                        candidate.lines().skip(new_index).take(new_len).collect();

                    let (orig_byte_start, orig_byte_end) =
                        trimmed_byte_range_for_lines(original, old_index, old_len);
                    let (cand_byte_start, cand_byte_end) =
                        trimmed_byte_range_for_lines(candidate, new_index, new_len);

                    let orig_node = orig_tree
                        .root_node()
                        .named_descendant_for_byte_range(orig_byte_start, orig_byte_end);
                    let cand_node = cand_tree
                        .root_node()
                        .named_descendant_for_byte_range(cand_byte_start, cand_byte_end);

                    let is_hunk_equivalent = match (orig_node, cand_node) {
                        (Some(on), Some(cn)) => are_nodes_semantically_equivalent(
                            &on,
                            original.as_bytes(),
                            &cn,
                            candidate.as_bytes(),
                        ),
                        _ => false,
                    };

                    if is_hunk_equivalent {
                        for l in &orig_slice {
                            reconciled_lines.push(l.to_string());
                        }
                        discarded_churn += old_len + new_len;
                    } else if let Some(import_reconciled) =
                        try_reconcile_import_block(self.lang, &orig_slice, &cand_slice)
                    {
                        for l in import_reconciled {
                            reconciled_lines.push(l);
                        }
                        discarded_churn += old_len + new_len;
                    } else if old_len == new_len {
                        for i in 0..old_len {
                            let old_l = orig_slice[i];
                            let new_l = cand_slice[i];

                            if old_l.trim() == new_l.trim()
                                || are_lines_semantically_equivalent(
                                    &mut parser,
                                    old_l,
                                    new_l,
                                )
                            {
                                reconciled_lines.push(old_l.to_string());
                                discarded_churn += 2;
                            } else {
                                let old_indent = get_indentation(old_l);
                                let new_trimmed = new_l.trim_start();
                                reconciled_lines.push(format!("{}{}", old_indent, new_trimmed));
                            }
                        }
                    } else {
                        let mut start_old = 0;
                        let mut start_new = 0;
                        while start_old < old_len && start_new < new_len {
                            if orig_slice[start_old].trim() == cand_slice[start_new].trim()
                                || are_lines_semantically_equivalent(
                                    &mut parser,
                                    orig_slice[start_old],
                                    cand_slice[start_new],
                                )
                            {
                                reconciled_lines.push(orig_slice[start_old].to_string());
                                discarded_churn += 2;
                                start_old += 1;
                                start_new += 1;
                            } else if is_comment_line(orig_slice[start_old])
                                && !is_comment_line(cand_slice[start_new])
                            {
                                // AI silently stripped an original comment: preserve it!
                                reconciled_lines.push(orig_slice[start_old].to_string());
                                discarded_churn += 1;
                                start_old += 1;
                            } else {
                                break;
                            }
                        }

                        let mut end_old = old_len;
                        let mut end_new = new_len;
                        while end_old > start_old
                            && end_new > start_new
                            && (orig_slice[end_old - 1].trim() == cand_slice[end_new - 1].trim()
                                || are_lines_semantically_equivalent(
                                    &mut parser,
                                    orig_slice[end_old - 1],
                                    cand_slice[end_new - 1],
                                ))
                        {
                            end_old -= 1;
                            end_new -= 1;
                        }

                        let middle_cand = &cand_slice[start_new..end_new];
                        let base_indent = if start_old < orig_slice.len() {
                            get_indentation(orig_slice[start_old])
                        } else {
                            ""
                        };

                        for l in middle_cand {
                            if l.trim().is_empty() {
                                reconciled_lines.push(String::new());
                            } else {
                                let cur_cand_indent = get_indentation(l);
                                if cur_cand_indent.is_empty() && !base_indent.is_empty() {
                                    reconciled_lines
                                        .push(format!("{}{}", base_indent, l.trim_start()));
                                } else {
                                    reconciled_lines.push(l.to_string());
                                }
                            }
                        }

                        for i in end_old..old_len {
                            reconciled_lines.push(orig_slice[i].to_string());
                            discarded_churn += 2;
                        }
                    }
                }
            }
        }

        let reconciled_text = reconciled_lines.join("\n");

        let final_tree = parser.parse(&reconciled_text, None);
        let final_text = match final_tree {
            Some(t) if !t.root_node().has_error() => reconciled_text,
            _ => candidate.to_string(),
        };

        let final_diff = TextDiff::from_lines(original, &final_text);
        let mut surgical_added = 0usize;
        let mut surgical_removed = 0usize;
        for c in final_diff.iter_all_changes() {
            match c.tag() {
                ChangeTag::Insert => surgical_added += 1,
                ChangeTag::Delete => surgical_removed += 1,
                ChangeTag::Equal => {}
            }
        }

        let estimated_tokens_saved = discarded_churn * 14;

        let stats = SurgicalStats {
            original_lines: original.lines().count(),
            candidate_lines: candidate.lines().count(),
            raw_added_lines: raw_added,
            raw_removed_lines: raw_removed,
            surgical_added_lines: surgical_added,
            surgical_removed_lines: surgical_removed,
            discarded_churn_lines: discarded_churn,
            estimated_tokens_saved,
        };

        (final_text, stats)
    }

    pub fn render_visual_diff(&self, original: &str, candidate: &str, reconciled: &str) -> String {
        use colored::*;
        let mut out = String::new();
        out.push_str(&format!("{}\n", "[StyleFreeze] Surgical Diff Breakdown:".bold().cyan()));
        out.push_str(&format!("{}\n", "------------------------------------------------------------".dimmed()));

        let diff_cand = TextDiff::from_lines(candidate, reconciled);
        let mut churn_count = 0;
        let mut applied_count = 0;

        for change in diff_cand.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    // Lines in candidate that StyleFreeze rejected
                    churn_count += 1;
                    out.push_str(&format!(" {} {}", "[-] REVERTED CHURN:".bright_red().bold(), change.value().trim_end()));
                    out.push('\n');
                }
                _ => {}
            }
        }

        let diff_orig = TextDiff::from_lines(original, reconciled);
        for change in diff_orig.iter_all_changes() {
            match change.tag() {
                ChangeTag::Insert => {
                    // Lines actually applied to original
                    applied_count += 1;
                    out.push_str(&format!(" {} {}", "[+] APPLIED CHANGE:".bright_green().bold(), change.value().trim_end()));
                    out.push('\n');
                }
                _ => {}
            }
        }

        out.push_str(&format!("{}\n", "------------------------------------------------------------".dimmed()));
        out.push_str(&format!(
            " {} {} lines of churn frozen, {} lines of logic applied.\n",
            "[OK]".bright_green().bold(),
            churn_count.to_string().bright_yellow().bold(),
            applied_count.to_string().bright_green().bold()
        ));

        out
    }

    fn fallback_stats(&self, original: &str, candidate: &str) -> (String, SurgicalStats) {
        let diff = TextDiff::from_lines(original, candidate);
        let mut added = 0;
        let mut removed = 0;
        for c in diff.iter_all_changes() {
            match c.tag() {
                ChangeTag::Insert => added += 1,
                ChangeTag::Delete => removed += 1,
                ChangeTag::Equal => {}
            }
        }
        (
            candidate.to_string(),
            SurgicalStats {
                original_lines: original.lines().count(),
                candidate_lines: candidate.lines().count(),
                raw_added_lines: added,
                raw_removed_lines: removed,
                surgical_added_lines: added,
                surgical_removed_lines: removed,
                discarded_churn_lines: 0,
                estimated_tokens_saved: 0,
            },
        )
    }
}

fn is_comment_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("//") || t.starts_with("/*") || t.starts_with('*') || t.starts_with('#')
}

fn is_import_or_include_line(lang: SupportedLang, line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() || is_comment_line(t) {
        return false;
    }

    match lang {
        SupportedLang::TypeScript | SupportedLang::JavaScript | SupportedLang::Tsx => {
            t.starts_with("import ")
                || t.starts_with("import{")
                || t.starts_with("import*")
                || t.starts_with("import(")
                || (t.starts_with("export ") && t.contains(" from "))
                || ((t.starts_with("const ") || t.starts_with("let ") || t.starts_with("var "))
                    && t.contains("require("))
        }
        SupportedLang::Python => {
            t.starts_with("import ") || (t.starts_with("from ") && t.contains(" import "))
        }
        SupportedLang::Rust => {
            t.starts_with("use ") && t.ends_with(';')
        }
        SupportedLang::Go => {
            t.starts_with("import ") || (t.starts_with('"') && t.ends_with('"'))
        }
        SupportedLang::C | SupportedLang::Cpp => {
            t.starts_with("#include")
        }
        SupportedLang::Json => false,
    }
}

fn normalize_import_key(line: &str) -> String {
    let mut s = line.trim().trim_end_matches(';').trim().to_string();
    s = s.replace('"', "'");
    let words: Vec<&str> = s.split_whitespace().collect();
    words.join(" ")
}

fn find_import_header_boundary(lang: SupportedLang, lines: &[&str]) -> Option<usize> {
    let mut last_import_line_idx = None;

    for (idx, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.is_empty() || is_comment_line(t) {
            continue;
        }

        if is_import_or_include_line(lang, t) {
            last_import_line_idx = Some(idx);
        } else {
            break;
        }
    }

    last_import_line_idx.map(|idx| idx + 1)
}

fn reconcile_import_header(
    lang: SupportedLang,
    original: &str,
    candidate: &str,
) -> (String, usize) {
    if lang == SupportedLang::Json {
        return (candidate.to_string(), 0);
    }

    let orig_lines: Vec<&str> = original.lines().collect();
    let cand_lines: Vec<&str> = candidate.lines().collect();

    let orig_boundary = match find_import_header_boundary(lang, &orig_lines) {
        Some(b) => b,
        None => return (candidate.to_string(), 0),
    };
    let cand_boundary = match find_import_header_boundary(lang, &cand_lines) {
        Some(b) => b,
        None => return (candidate.to_string(), 0),
    };

    let orig_header = &orig_lines[..orig_boundary];
    let cand_header = &cand_lines[..cand_boundary];

    let orig_active: Vec<&str> = orig_header
        .iter()
        .copied()
        .filter(|l| is_import_or_include_line(lang, l))
        .collect();
    let cand_active: Vec<&str> = cand_header
        .iter()
        .copied()
        .filter(|l| is_import_or_include_line(lang, l))
        .collect();

    let orig_keys: HashSet<String> = orig_active.iter().map(|l| normalize_import_key(l)).collect();
    let cand_keys: HashSet<String> = cand_active.iter().map(|l| normalize_import_key(l)).collect();

    if orig_keys == cand_keys {
        let mut new_lines: Vec<&str> = Vec::new();
        new_lines.extend_from_slice(orig_header);
        new_lines.extend_from_slice(&cand_lines[cand_boundary..]);
        let joined = new_lines.join("\n");
        let result = if candidate.ends_with('\n') {
            joined + "\n"
        } else {
            joined
        };
        return (result, cand_header.len() + orig_header.len());
    }

    if orig_keys.is_subset(&cand_keys) {
        let mut new_lines: Vec<String> = orig_header.iter().map(|l| l.to_string()).collect();
        for cand_line in &cand_active {
            let key = normalize_import_key(cand_line);
            if !orig_keys.contains(&key) {
                new_lines.push(cand_line.to_string());
            }
        }
        for remaining in &cand_lines[cand_boundary..] {
            new_lines.push(remaining.to_string());
        }
        let joined = new_lines.join("\n");
        let result = if candidate.ends_with('\n') {
            joined + "\n"
        } else {
            joined
        };
        return (result, cand_header.len());
    }

    (candidate.to_string(), 0)
}

fn try_reconcile_import_block(
    lang: SupportedLang,
    orig_slice: &[&str],
    cand_slice: &[&str],
) -> Option<Vec<String>> {
    let orig_active: Vec<&str> = orig_slice
        .iter()
        .copied()
        .filter(|l| !l.trim().is_empty() && !is_comment_line(l))
        .collect();
    let cand_active: Vec<&str> = cand_slice
        .iter()
        .copied()
        .filter(|l| !l.trim().is_empty() && !is_comment_line(l))
        .collect();

    if orig_active.is_empty() || cand_active.is_empty() {
        return None;
    }

    if !orig_active.iter().all(|l| is_import_or_include_line(lang, l)) {
        return None;
    }
    if !cand_active.iter().all(|l| is_import_or_include_line(lang, l)) {
        return None;
    }

    let orig_keys: HashSet<String> = orig_active.iter().map(|l| normalize_import_key(l)).collect();
    let cand_keys: HashSet<String> = cand_active.iter().map(|l| normalize_import_key(l)).collect();

    if orig_keys == cand_keys {
        return Some(orig_slice.iter().map(|s| s.to_string()).collect());
    }

    if orig_keys.is_subset(&cand_keys) {
        let mut result: Vec<String> = orig_slice.iter().map(|s| s.to_string()).collect();
        for cand_line in &cand_active {
            let key = normalize_import_key(cand_line);
            if !orig_keys.contains(&key) {
                result.push(cand_line.to_string());
            }
        }
        return Some(result);
    }

    None
}

fn are_lines_semantically_equivalent(
    parser: &mut Parser,
    line_a: &str,
    line_b: &str,
) -> bool {
    let ta = line_a.trim();
    let tb = line_b.trim();
    if ta == tb {
        return true;
    }

    if let (Some(tree_a), Some(tree_b)) = (parser.parse(ta, None), parser.parse(tb, None)) {
        if !tree_a.root_node().has_error() && !tree_b.root_node().has_error() {
            return are_nodes_semantically_equivalent(
                &tree_a.root_node(),
                ta.as_bytes(),
                &tree_b.root_node(),
                tb.as_bytes(),
            );
        }
    }

    false
}

fn trimmed_byte_range_for_lines(text: &str, line_idx: usize, count: usize) -> (usize, usize) {
    let mut current_line = 0;
    let mut start_opt = None;
    let mut end_opt = None;

    let mut byte_offset = 0;
    for line in text.split_inclusive('\n') {
        if current_line == line_idx {
            let non_ws = line.find(|c: char| !c.is_whitespace()).unwrap_or(0);
            start_opt = Some(byte_offset + non_ws);
        }
        if current_line == line_idx + count - 1 {
            let trimmed_end = line.trim_end_matches(['\r', '\n']).len();
            end_opt = Some(byte_offset + trimmed_end);
            break;
        }
        byte_offset += line.len();
        current_line += 1;
    }

    (start_opt.unwrap_or(0), end_opt.unwrap_or(text.len()))
}

fn get_indentation(line: &str) -> &str {
    let non_ws = line.find(|c: char| !c.is_whitespace()).unwrap_or(line.len());
    &line[..non_ws]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surgical_reconciliation_preserves_indent_and_filters_churn() {
        let original = r#"function calculateDiscount(price: number, isVip: boolean): number {
    if (isVip) {
        return price * 0.8;
    }
    return price;
}

function sendNotification(msg: string) {
    console.log("LOG: " + msg);
}
"#;

        let candidate = r#"function calculateDiscount(price: number, isVip: boolean): number {
  if (isVip) {
    return price * 0.7;
  }
  return price;
}

function sendNotification(msg: string) {
  console.log("LOG: " + msg);
}
"#;

        let engine = SurgicalEngine::new(SupportedLang::TypeScript);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("Result:\n{}", result);
        println!("Stats: {:?}", stats);

        assert!(result.contains("return price * 0.7;"));
        assert!(result.contains("    console.log(\"LOG: \" + msg);"));
        assert!(result.contains("    if (isVip) {"));
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_python_surgical_reconciliation() {
        let original = r#"def get_user_tier(score: int) -> str:
    # Check gold threshold
    if score >= 100:
        return "gold"
    return "standard"

def format_greeting(name: str) -> str:
    return f"Hello, {name}!"
"#;

        let candidate = r#"def get_user_tier(score: int) -> str:
  if score >= 90:
    return 'gold'
  return 'standard'

def format_greeting(name: str) -> str:
  return f'Hello, {name}!'
"#;

        let engine = SurgicalEngine::new(SupportedLang::Python);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("Python Result:\n{}", result);
        println!("Stats: {:?}", stats);

        assert!(result.contains("score >= 90:"));
        assert!(result.contains("    return f\"Hello, {name}!\""));
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_rust_surgical_reconciliation() {
        let original = r#"pub fn process_order(id: u64, amount: f64) -> bool {
    // Audit threshold
    if amount > 1000.0 {
        return false;
    }
    true
}

pub fn format_id(id: u64) -> String {
    format!("ID: {}", id)
}
"#;

        let candidate = r#"pub fn process_order(id: u64, amount: f64) -> bool {
  if amount > 5000.0 {
    return false;
  }
  true
}

pub fn format_id(id: u64) -> String {
  format!("ID: {}", id)
}
"#;

        let engine = SurgicalEngine::new(SupportedLang::Rust);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("Rust Result:\n{}", result);
        assert!(result.contains("amount > 5000.0"));
        // format_id must retain 4 spaces
        assert!(result.contains("    format!(\"ID: {}\", id)"));
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_go_surgical_reconciliation_preserves_tabs() {
        let original = "package main\n\nfunc CalculateTax(income float64) float64 {\n\tif income > 50000 {\n\t\treturn income * 0.2\n\t}\n\treturn income * 0.1\n}\n\nfunc Version() string {\n\treturn \"v1.0\"\n}\n";

        // AI converted tabs to 2 spaces and changed tax to 0.25
        let candidate = "package main\n\nfunc CalculateTax(income float64) float64 {\n  if income > 50000 {\n    return income * 0.25\n  }\n  return income * 0.1\n}\n\nfunc Version() string {\n  return \"v1.0\"\n}\n";

        let engine = SurgicalEngine::new(SupportedLang::Go);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("Go Result:\n{}", result);
        assert!(result.contains("income * 0.25"));
        // Version must preserve tab indentation
        assert!(result.contains("\treturn \"v1.0\""));
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_unsolicited_comment_deletion_is_prevented() {
        let original = r#"export function validateEmail(email: string): boolean {
    // Validate email structure
    if (!email.includes("@")) {
        return false;
    }
    return true;
}
"#;

        let candidate = r#"export function validateEmail(email: string): boolean {
  if (!email.includes("@") || email.length < 5) {
    return false;
  }
  return true;
}
"#;

        let engine = SurgicalEngine::new(SupportedLang::TypeScript);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("Result:\n{}", result);
        assert!(result.contains("// Validate email structure"));
        assert!(result.contains("email.length < 5"));
        assert!(result.contains("    return false;"));
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_json_surgical_reconciliation() {
        let original = r#"{
  "name": "my-app",
  "version": "1.0.0",
  "scripts": {
    "test": "cargo test"
  }
}
"#;

        // Candidate changed test command, but re-indented whole file to 4 spaces
        let candidate = r#"{
    "name": "my-app",
    "version": "1.0.0",
    "scripts": {
        "test": "cargo test --all"
    }
}
"#;

        let engine = SurgicalEngine::new(SupportedLang::Json);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("JSON Result:\n{}", result);
        assert!(result.contains("\"test\": \"cargo test --all\""));
        // Name and version must retain 2 spaces
        assert!(result.contains("  \"name\": \"my-app\""));
        assert!(result.contains("  \"version\": \"1.0.0\""));
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_c_surgical_reconciliation() {
        let original = r#"#include <stdio.h>

int calculate_buffer_size(int packet_len) {
    // Ensure padding for header
    if (packet_len < 64) {
        return 64;
    }
    return packet_len + 16;
}

void log_packet(const char* msg) {
    printf("[PACKET] %s\n", msg);
}
"#;

        // Candidate changed 64 -> 128, but reformatted whole file to 2 spaces
        let candidate = r#"#include <stdio.h>

int calculate_buffer_size(int packet_len) {
  if (packet_len < 128) {
    return 128;
  }
  return packet_len + 16;
}

void log_packet(const char* msg) {
  printf("[PACKET] %s\n", msg);
}
"#;

        let engine = SurgicalEngine::new(SupportedLang::C);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("C Result:\n{}", result);
        assert!(result.contains("packet_len < 128"));
        // log_packet must retain 4 spaces
        assert!(result.contains("    printf(\"[PACKET] %s\\n\", msg);"));
        // calculate_buffer_size comment must be preserved
        assert!(result.contains("// Ensure padding for header"));
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_cpp_surgical_reconciliation() {
        let original = r#"#include <string>

namespace core {
    class SessionManager {
    public:
        bool isValid(int id) {
            // Check session ID positive
            if (id <= 0) {
                return false;
            }
            return true;
        }
    };
}
"#;

        // Candidate changed <= 0 to < 0, but reformatted to 2 spaces
        let candidate = r#"#include <string>

namespace core {
  class SessionManager {
  public:
    bool isValid(int id) {
      if (id < 0) {
        return false;
      }
      return true;
    }
  };
}
"#;

        let engine = SurgicalEngine::new(SupportedLang::Cpp);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("C++ Result:\n{}", result);
        assert!(result.contains("id < 0"));
        assert!(result.contains("// Check session ID positive"));
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_import_shield_prevents_reordering_churn() {
        let original = r#"import { z } from "./z";
import { a } from "./a";
import { m } from "./m";

export function run() {
    return 42;
}
"#;

        // Candidate shuffled imports alphabetically and used single quotes
        let candidate = r#"import { a } from './a';
import { m } from './m';
import { z } from './z';

export function run() {
    return 42;
}
"#;

        let engine = SurgicalEngine::new(SupportedLang::TypeScript);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("Reconciled TS Imports:\n{}", result);
        assert_eq!(result.trim(), original.trim());
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_import_shield_appends_new_import_preserving_order() {
        let original = r#"import { z } from "./z";
import { a } from "./a";

export function run() {
    return 42;
}
"#;

        // Candidate shuffled existing and inserted new import
        let candidate = r#"import { a } from "./a";
import { newlyAdded } from "./new";
import { z } from "./z";

export function run() {
    return 42;
}
"#;

        let engine = SurgicalEngine::new(SupportedLang::TypeScript);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("Appended TS Imports:\n{}", result);
        // Original order preserved: z, then a
        let idx_z = result.find("from \"./z\"").unwrap();
        let idx_a = result.find("from \"./a\"").unwrap();
        let idx_new = result.find("from \"./new\"").unwrap();

        assert!(idx_z < idx_a);
        assert!(idx_a < idx_new);
        assert!(stats.discarded_churn_lines > 0);
    }

    #[test]
    fn test_python_import_shield_preserves_custom_grouping() {
        let original = r#"import sys
import os

from collections import defaultdict

def execute():
    pass
"#;

        let candidate = r#"from collections import defaultdict
import os
import sys

def execute():
    pass
"#;

        let engine = SurgicalEngine::new(SupportedLang::Python);
        let (result, stats) = engine.reconcile(original, candidate);

        println!("Python Reconciled Imports:\n{}", result);
        assert_eq!(result.trim(), original.trim());
        assert!(stats.discarded_churn_lines > 0);
    }
}
