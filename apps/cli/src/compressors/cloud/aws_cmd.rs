use anyhow::{Context, Result};
use serde_json::Value;

/// Extract the `Name` tag value from an instance's Tags array.
fn extract_name_tag(tags: &Value) -> Option<String> {
    tags.as_array()?.iter().find_map(|tag| {
        let key = tag.get("Key")?.as_str()?;
        let value = tag.get("Value")?.as_str()?;
        if key == "Name" {
            Some(value.to_string())
        } else {
            None
        }
    })
}

/// Extract a string field from a JSON value, returning "-" if missing/null/empty.
fn field_str<'a>(obj: &'a Value, key: &str) -> &'a str {
    obj.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("-")
}

/// Filter `aws ec2 describe-instances` JSON output.
///
/// Parses the JSON, extracts key fields per instance:
///   InstanceId, InstanceType, State, PublicIpAddress, Name tag
/// and formats them as a compact human-readable table.
///
/// Falls back to the original input if JSON parsing fails.
///
/// Target: >=80% token savings.
pub fn filter_aws_json(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    match try_filter_aws_json(input) {
        Ok(output) => output,
        Err(_) => input.to_string(),
    }
}

fn try_filter_aws_json(input: &str) -> Result<String> {
    let root: Value = serde_json::from_str(input).context("Failed to parse aws ec2 JSON")?;

    let reservations = root
        .get("Reservations")
        .and_then(|r| r.as_array())
        .context("Missing Reservations array")?;

    struct InstanceRow {
        id: String,
        instance_type: String,
        state: String,
        public_ip: String,
        name: String,
    }

    let mut rows: Vec<InstanceRow> = Vec::new();

    for reservation in reservations {
        let instances = match reservation.get("Instances").and_then(|i| i.as_array()) {
            Some(i) => i,
            None => continue,
        };

        for instance in instances {
            let id = field_str(instance, "InstanceId").to_string();
            let instance_type = field_str(instance, "InstanceType").to_string();
            let state = instance
                .get("State")
                .and_then(|s| s.get("Name"))
                .and_then(|n| n.as_str())
                .unwrap_or("-")
                .to_string();
            let public_ip = field_str(instance, "PublicIpAddress").to_string();
            let name = instance
                .get("Tags")
                .and_then(extract_name_tag)
                .unwrap_or_else(|| "-".to_string());

            rows.push(InstanceRow {
                id,
                instance_type,
                state,
                public_ip,
                name,
            });
        }
    }

    if rows.is_empty() {
        return Ok(input.to_string());
    }

    // Build compact table header + rows
    let mut out = String::new();
    out.push_str("INSTANCE_ID          TYPE       STATE    PUBLIC_IP      NAME\n");
    for row in &rows {
        out.push_str(&format!(
            "{:<20} {:<10} {:<8} {:<14} {}\n",
            row.id, row.instance_type, row.state, row.public_ip, row.name
        ));
    }

    Ok(out.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_aws_ec2_format() {
        let input = include_str!("../../../tests/fixtures/cloud/aws_ec2_raw.txt");
        let output = filter_aws_json(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_aws_ec2_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/aws_ec2_raw.txt");
        let output = filter_aws_json(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 80.0,
            "Expected >=80% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_aws_ec2_empty() {
        assert_eq!(filter_aws_json(""), "");
    }

    #[test]
    fn test_aws_ec2_malformed() {
        let output = filter_aws_json("not json at all\njust random text");
        // Should fall back to input unchanged
        assert!(output.contains("not json at all"));
    }

    #[test]
    fn test_aws_ec2_keeps_instance_ids() {
        let input = include_str!("../../../tests/fixtures/cloud/aws_ec2_raw.txt");
        let output = filter_aws_json(input);
        assert!(
            output.contains("i-0a1b2c3d4e5f67890"),
            "Should include first instance ID"
        );
        assert!(
            output.contains("i-0c3d4e5f67890123"),
            "Should include third instance ID"
        );
    }

    #[test]
    fn test_aws_ec2_keeps_name_tags() {
        let input = include_str!("../../../tests/fixtures/cloud/aws_ec2_raw.txt");
        let output = filter_aws_json(input);
        assert!(
            output.contains("web-server-prod-1"),
            "Should include Name tag"
        );
        assert!(output.contains("dev-bastion"), "Should include dev-bastion");
    }

    #[test]
    fn test_aws_ec2_strips_verbose_fields() {
        let input = include_str!("../../../tests/fixtures/cloud/aws_ec2_raw.txt");
        let output = filter_aws_json(input);
        assert!(
            !output.contains("BlockDeviceMappings"),
            "Should strip BlockDeviceMappings"
        );
        assert!(
            !output.contains("PrivateDnsName"),
            "Should strip PrivateDnsName"
        );
        assert!(
            !output.contains("ReservationId"),
            "Should strip ReservationId"
        );
    }

    #[test]
    fn test_aws_ec2_shows_stopped_instance() {
        let input = include_str!("../../../tests/fixtures/cloud/aws_ec2_raw.txt");
        let output = filter_aws_json(input);
        assert!(
            output.contains("stopped"),
            "Should show stopped instance state"
        );
    }

    #[test]
    fn test_aws_ec2_invalid_json_fallback() {
        let malformed = r#"{"Reservations": "not-an-array"}"#;
        let output = filter_aws_json(malformed);
        // Should fall back to original input
        assert!(output.contains("not-an-array"));
    }

    #[test]
    fn test_aws_json_snapshot() {
        let raw = include_str!("../../../tests/fixtures/cloud/aws_ec2_raw.txt");
        let out = filter_aws_json(raw);
        insta::assert_snapshot!(out);
    }

    #[test]
    fn test_aws_json_savings_at_least_60pct() {
        let raw = include_str!("../../../tests/fixtures/cloud/aws_ec2_raw.txt");
        let in_t = raw.split_whitespace().count();
        let out_t = filter_aws_json(raw).split_whitespace().count();
        let pct = 100 - (out_t * 100 / in_t.max(1));
        assert!(pct >= 60, "expected ≥60%, got {}%", pct);
    }
}
