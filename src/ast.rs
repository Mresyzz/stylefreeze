use tree_sitter::Node;

fn is_ignorable(kind: &str) -> bool {
    matches!(kind, ";" | "comment" | "\"" | "'")
}

fn normalize_string_literal(raw: &str) -> &str {
    let s = raw.trim();
    if (s.starts_with('"') && s.ends_with('"'))
        || (s.starts_with('\'') && s.ends_with('\''))
        || (s.starts_with('`') && s.ends_with('`'))
    {
        if s.len() >= 2 {
            &s[1..s.len() - 1]
        } else {
            s
        }
    } else {
        s
    }
}

fn normalize_string_prefix(s: &str) -> String {
    let mut prefix = String::new();
    for c in s.chars() {
        if c.is_alphabetic() {
            prefix.push(c.to_ascii_lowercase());
        }
    }
    prefix
}

pub fn are_nodes_semantically_equivalent(
    node_a: &Node,
    source_a: &[u8],
    node_b: &Node,
    source_b: &[u8],
) -> bool {
    if node_a.kind() != node_b.kind() {
        return false;
    }

    let text_a = &source_a[node_a.byte_range()];
    let text_b = &source_b[node_b.byte_range()];

    // Leaf node
    if node_a.child_count() == 0 && node_b.child_count() == 0 {
        let str_a = std::str::from_utf8(text_a).unwrap_or("");
        let str_b = std::str::from_utf8(text_b).unwrap_or("");

        // Python tree-sitter string boundaries
        if node_a.kind() == "string_start" {
            return normalize_string_prefix(str_a) == normalize_string_prefix(str_b);
        }
        if node_a.kind() == "string_end" {
            return true;
        }

        if node_a.kind() == "string" || node_a.kind() == "string_fragment" {
            return normalize_string_literal(str_a) == normalize_string_literal(str_b);
        }

        return str_a.trim() == str_b.trim();
    }

    let mut cursor_a = node_a.walk();
    let mut cursor_b = node_b.walk();

    let children_a: Vec<Node> = node_a
        .children(&mut cursor_a)
        .filter(|n| !is_ignorable(n.kind()))
        .collect();

    let children_b: Vec<Node> = node_b
        .children(&mut cursor_b)
        .filter(|n| !is_ignorable(n.kind()))
        .collect();

    if children_a.len() != children_b.len() {
        return false;
    }

    for (ca, cb) in children_a.iter().zip(children_b.iter()) {
        if !are_nodes_semantically_equivalent(ca, source_a, cb, source_b) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    #[test]
    fn test_semantically_equivalent_with_format_diff() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();

        let code1 = "function add(a: number, b: number) {\n    return a + b;\n}";
        let code2 = "function add(a: number, b: number) {\n  return a + b\n}";

        let tree1 = parser.parse(code1, None).unwrap();
        let tree2 = parser.parse(code2, None).unwrap();

        assert!(are_nodes_semantically_equivalent(
            &tree1.root_node(),
            code1.as_bytes(),
            &tree2.root_node(),
            code2.as_bytes()
        ));
    }

    #[test]
    fn test_semantic_difference_detected() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();

        let code1 = "function add(a: number, b: number) { return a + b; }";
        let code2 = "function add(a: number, b: number) { return a - b; }";

        let tree1 = parser.parse(code1, None).unwrap();
        let tree2 = parser.parse(code2, None).unwrap();

        assert!(!are_nodes_semantically_equivalent(
            &tree1.root_node(),
            code1.as_bytes(),
            &tree2.root_node(),
            code2.as_bytes()
        ));
    }

    #[test]
    fn test_python_fstring_quote_equivalence() {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .unwrap();

        let code1 = "def greet(name: str) -> str:\n    return f\"Hello, {name}!\"\n";
        let code2 = "def greet(name: str) -> str:\n  return f'Hello, {name}!'\n";

        let tree1 = parser.parse(code1, None).unwrap();
        let tree2 = parser.parse(code2, None).unwrap();

        assert!(are_nodes_semantically_equivalent(
            &tree1.root_node(),
            code1.as_bytes(),
            &tree2.root_node(),
            code2.as_bytes()
        ));
    }
}
