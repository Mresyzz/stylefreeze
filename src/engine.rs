use crate::ast::are_nodes_semantically_equivalent;
use crate::lang::SupportedLang;
use similar::{ChangeTag, DiffOp, TextDiff};
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
        let mut discarded_churn = 0usize;

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
                        let mut start = 0;
                        while start < old_len
                            && start < new_len
                            && orig_slice[start].trim() == cand_slice[start].trim()
                        {
                            reconciled_lines.push(orig_slice[start].to_string());
                            discarded_churn += 2;
                            start += 1;
                        }

                        let mut end_old = old_len;
                        let mut end_new = new_len;
                        while end_old > start
                            && end_new > start
                            && orig_slice[end_old - 1].trim() == cand_slice[end_new - 1].trim()
                        {
                            end_old -= 1;
                            end_new -= 1;
                        }

                        let middle_cand = &cand_slice[start..end_new];
                        let base_indent = if start < orig_slice.len() {
                            get_indentation(orig_slice[start])
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
}
