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
    serde_json::to_string_pretty(&truncated).context("failed to serialize JSON")
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
