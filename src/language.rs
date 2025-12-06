use regex::Regex;
use tree_sitter::{Parser, Query, QueryCursor};
use streaming_iterator::StreamingIterator;

pub mod comments {
    use super::*;

    pub fn remove_comments(content: &str, extension: &str) -> Option<String> {
        let pattern = match extension {
            "rs" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp" => {
                // C-style comments: // ... and /* ... */
                r"(?s)//.*?\n|/\*.*?\*/"
            },
            "py" | "sh" | "yaml" | "yml" | "toml" | "rb" | "pl" => {
                // Hash-style comments: # ...
                r"#.*"
            },
            _ => return None,
        };

        if let Ok(re) = Regex::new(pattern) {
             Some(re.replace_all(content, "").to_string())
        } else {
             None
        }
    }
}

pub mod compression {
    use super::*;

    pub fn compress_content(content: &str, extension: &str) -> Option<String> {
        let mut parser = Parser::new();

        let (language, query_str) = match extension {
            "rs" => (tree_sitter_rust::LANGUAGE.into(), RUST_QUERY),
            "ts" | "tsx" => (tree_sitter_typescript::LANGUAGE_TSX.into(), TS_QUERY),
            "js" | "jsx" => (tree_sitter_javascript::LANGUAGE.into(), JS_QUERY),
            "py" => (tree_sitter_python::LANGUAGE.into(), PYTHON_QUERY),
            "go" => (tree_sitter_go::LANGUAGE.into(), GO_QUERY),
            _ => return None, // Language not supported for compression
        };

        parser.set_language(&language).ok()?;
        let tree = parser.parse(content, None)?;
        let query = Query::new(&language, query_str).ok()?;
        let mut cursor = QueryCursor::new();

        // We collect ranges of "essential" code (signatures, headers)
        let mut ranges = Vec::new();

        let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;
                ranges.push(node.byte_range());
            }
        }

        if ranges.is_empty() {
            return Some(content.to_string()); // Fallback if no definitions found
        }

        // Sort and merge overlapping ranges
        ranges.sort_by(|a, b| a.start.cmp(&b.start));

        let mut merged_ranges = Vec::new();
        let mut current_range = ranges[0].clone();

        for next in ranges.into_iter().skip(1) {
            if next.start <= current_range.end {
                current_range.end = std::cmp::max(current_range.end, next.end);
            } else {
                merged_ranges.push(current_range);
                current_range = next;
            }
        }
        merged_ranges.push(current_range);

        // Reconstruct content
        let mut result = String::new();
        let bytes = content.as_bytes();
        let separator = "\n// ... [implementation details hidden] ...\n";

        for range in merged_ranges {
            let chunk = String::from_utf8_lossy(&bytes[range.start..range.end]);
            if !result.is_empty() {
                result.push_str(separator);
            }
            result.push_str(chunk.trim());
        }

        Some(result)
    }

    // Simplified queries to capture definitions/signatures
    const RUST_QUERY: &str = r#"
        (function_item) @f
        (impl_item) @i
        (struct_item) @s
        (enum_item) @e
        (trait_item) @t
        (mod_item) @m
    "#;

    const TS_QUERY: &str = r#"
        (function_declaration) @f
        (class_declaration) @c
        (interface_declaration) @i
        (type_alias_declaration) @t
        (enum_declaration) @e
        (method_definition) @m
        (abstract_class_declaration) @ac
        (module) @mod
    "#;

    const JS_QUERY: &str = r#"
        (function_declaration) @f
        (class_declaration) @c
        (method_definition) @m
    "#;

    const PYTHON_QUERY: &str = r#"
        (function_definition) @f
        (class_definition) @c
    "#;

    const GO_QUERY: &str = r#"
        (function_declaration) @f
        (method_declaration) @m
        (type_declaration) @t
    "#;
}
