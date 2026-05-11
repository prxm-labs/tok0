//! JSON-aware truncation compressor.
//!
//! Truncates large arrays (> [`ARRAY_TRUNCATE_AT`]) and objects (> [`OBJECT_TRUNCATE_AT`])
//! while preserving structural validity — output is always valid JSON.

use anyhow::{Context, Result};
use serde_json::Value;

const ARRAY_TRUNCATE_AT: usize = 16;
const OBJECT_TRUNCATE_AT: usize = 32;

pub fn filter_json(input: &str) -> Result<String> {
    let v: Value = serde_json::from_str(input.trim()).context("input is not valid JSON")?;
    let truncated = truncate(&v);
    format_top_level(&truncated)
}

/// Serialize as a single line per top-level array element or object pair,
/// with inner structure minified (no indentation). Top-level scalars and
/// empty containers produce single-line output. This keeps the result
/// scannable line-by-line for the LLM while shedding the bulk of the
/// whitespace overhead of `to_string_pretty`.
fn format_top_level(v: &Value) -> Result<String> {
    match v {
        Value::Array(arr) if !arr.is_empty() => {
            let mut parts: Vec<String> = Vec::with_capacity(arr.len());
            for elem in arr {
                parts.push(
                    serde_json::to_string(elem).context("failed to serialize array element")?,
                );
            }
            Ok(format!("[\n{}\n]", parts.join(",\n")))
        }
        Value::Object(obj) if !obj.is_empty() => {
            let mut parts: Vec<String> = Vec::with_capacity(obj.len());
            for (k, val) in obj {
                let key = serde_json::to_string(k).context("failed to serialize object key")?;
                let val_str =
                    serde_json::to_string(val).context("failed to serialize object value")?;
                parts.push(format!("{key}:{val_str}"));
            }
            Ok(format!("{{\n{}\n}}", parts.join(",\n")))
        }
        other => serde_json::to_string(other).context("failed to serialize JSON"),
    }
}

fn truncate(v: &Value) -> Value {
    match v {
        Value::Array(arr) => {
            if arr.len() > ARRAY_TRUNCATE_AT {
                let mut out: Vec<Value> =
                    arr.iter().take(ARRAY_TRUNCATE_AT).map(truncate).collect();
                out.push(Value::String(format!(
                    "…{} more…",
                    arr.len() - ARRAY_TRUNCATE_AT
                )));
                Value::Array(out)
            } else {
                Value::Array(arr.iter().map(truncate).collect())
            }
        }
        Value::Object(obj) => {
            if obj.len() > OBJECT_TRUNCATE_AT {
                let mut out = serde_json::Map::new();
                for (k, val) in obj.iter().take(OBJECT_TRUNCATE_AT) {
                    out.insert(k.clone(), truncate(val));
                }
                out.insert(
                    "__truncated__".to_string(),
                    Value::String(format!("…{} more keys…", obj.len() - OBJECT_TRUNCATE_AT)),
                );
                Value::Object(out)
            } else {
                Value::Object(obj.iter().map(|(k, v)| (k.clone(), truncate(v))).collect())
            }
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_json_small_passes_through() {
        let raw = include_str!("../../../tests/fixtures/system/json_small_raw.txt");
        let out = filter_json(raw).expect("should parse small JSON");
        assert!(out.contains("alice"));
        assert!(out.contains("hiking"));
    }

    #[test]
    fn test_json_nested_snapshot() {
        let raw = include_str!("../../../tests/fixtures/system/json_nested_raw.txt");
        insta::assert_snapshot!(filter_json(raw).expect("should parse nested JSON"));
    }

    #[test]
    fn test_json_huge_array_snapshot() {
        let raw = include_str!("../../../tests/fixtures/system/json_huge_array_raw.txt");
        insta::assert_snapshot!(filter_json(raw).expect("should parse huge array JSON"));
    }

    #[test]
    fn test_json_huge_array_savings() {
        let raw = include_str!("../../../tests/fixtures/system/json_huge_array_raw.txt");
        let out = filter_json(raw).expect("should parse huge array JSON");
        let pct = 100 - (count_tokens(&out) * 100 / count_tokens(raw).max(1));
        assert!(
            pct >= 90,
            "huge array should give >=90% savings, got {}%",
            pct
        );
    }

    #[test]
    fn test_json_invalid_input_errors() {
        assert!(filter_json("not json").is_err());
    }

    #[test]
    fn test_json_edge_cases() {
        assert!(filter_json("[]").expect("empty array").contains("[]"));
        assert!(filter_json("{}").expect("empty object").contains("{}"));
        assert!(filter_json("null").expect("null literal").contains("null"));
    }

    // ── Phase 1: minified JSON with top-level newlines ──────────────

    #[test]
    fn test_minified_array_one_element_per_line() {
        let raw = r#"[1, 2, 3]"#;
        let out = filter_json(raw).expect("parse small array");
        // Top-level array → one element per line, no inner indentation.
        let body_lines: Vec<&str> = out.lines().collect();
        assert!(body_lines.len() >= 5, "got: {out}");
        assert!(out.starts_with('['));
        assert!(out.ends_with(']'));
        // Lines should not start with 2-space pretty-print indent.
        for line in &body_lines {
            assert!(
                !line.starts_with("  "),
                "minified output must not contain pretty-print indent: '{line}'"
            );
        }
    }

    #[test]
    fn test_minified_object_one_pair_per_line() {
        let raw = r#"{"a": 1, "b": 2}"#;
        let out = filter_json(raw).expect("parse small object");
        assert!(out.starts_with('{'));
        assert!(out.ends_with('}'));
        for line in out.lines() {
            assert!(
                !line.starts_with("  "),
                "minified output must not contain pretty-print indent: '{line}'"
            );
        }
    }

    #[test]
    fn test_minified_inner_structure_flat() {
        // Nested arrays/objects inside top-level entries stay on one line.
        let raw = r#"{"users": [{"id": 1, "tags": ["a", "b"]}, {"id": 2}]}"#;
        let out = filter_json(raw).expect("parse nested");
        // The "users" line must contain the full inner array on one line.
        let users_line = out
            .lines()
            .find(|l| l.contains("users"))
            .expect("users line");
        assert!(users_line.contains("[{"));
        assert!(users_line.contains("}]"));
    }

    #[test]
    fn test_minified_savings_higher_than_pretty() {
        // Compare new vs old format on the huge array fixture.
        let raw = include_str!("../../../tests/fixtures/system/json_huge_array_raw.txt");
        let new_out = filter_json(raw).expect("filter");
        let v: Value = serde_json::from_str(raw.trim()).expect("parse");
        let old_out = serde_json::to_string_pretty(&truncate(&v)).expect("pretty");
        assert!(
            count_tokens(&new_out) <= count_tokens(&old_out),
            "minified should not lose ground: new {} vs old {}",
            count_tokens(&new_out),
            count_tokens(&old_out)
        );
    }

    #[test]
    fn test_minified_output_still_valid_json() {
        let raw = include_str!("../../../tests/fixtures/system/json_huge_array_raw.txt");
        let out = filter_json(raw).expect("filter");
        let _: Value = serde_json::from_str(&out)
            .expect("minified output with line breaks must still parse as JSON");
    }

    use proptest::prelude::*;
    use serde_json::json;
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]
        #[test]
        fn prop_json_output_is_always_valid_json(seed in 0u64..1000) {
            let arr: Vec<u64> = (0..seed % 200).collect();
            let v = json!({ "k": arr });
            let raw = serde_json::to_string(&v).expect("serialize input");
            let out = filter_json(&raw).expect("filter should succeed on valid JSON");
            let _: Value = serde_json::from_str(&out).expect("output must be valid JSON");
        }
    }
}
