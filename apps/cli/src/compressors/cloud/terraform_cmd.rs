use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // Lines like: "aws_vpc.main: Refreshing state... [id=vpc-0a1b2c3d4e5f67890]"
    static ref REFRESH_RE: Regex =
        Regex::new(r"^\S+: Refreshing state\.\.\.").unwrap();

    // Resource header: "  # aws_instance.main will be created" etc.
    static ref RESOURCE_HEADER_RE: Regex =
        Regex::new(r"^\s+#\s+\S+\s+will be\s+\w+").unwrap();

    // Resource block opener: "  + resource "aws_instance" "main" {"
    static ref RESOURCE_BLOCK_OPEN_RE: Regex =
        Regex::new(r#"^\s+[+~\-]\s+resource\s+"[^"]+"\s+"[^"]+"\s+\{"#).unwrap();

    // Attribute lines with "(known after apply)" — pure noise
    static ref KNOWN_AFTER_APPLY_RE: Regex =
        Regex::new(r"\(known after apply\)").unwrap();

    // Plan summary line: "Plan: N to add, N to change, N to destroy."
    static ref PLAN_SUMMARY_RE: Regex =
        Regex::new(r"^Plan:").unwrap();

    // Blank/separator lines inside blocks we want to keep
    static ref BLOCK_CLOSE_RE: Regex =
        Regex::new(r"^\s+\}\s*$").unwrap();

    // Tags block opener: "      + tags = {"
    static ref TAGS_OPEN_RE: Regex =
        Regex::new(r#"^\s+[+~\-]\s+tags\s+="#).unwrap();

    // Progress/status lines: "aws_instance.main: Still creating... [10s elapsed]"
    static ref STILL_PROGRESS_RE: Regex =
        Regex::new(r"^\S+: Still (?:creating|modifying|destroying)\.\.\.").unwrap();

    // Creating/modifying/destroying start lines:
    // "aws_security_group.main: Creating..."
    static ref ACTION_START_RE: Regex =
        Regex::new(r"^\S+: (?:Creating|Modifying|Destroying)\.\.\.$").unwrap();

    // Completion lines:
    // "aws_instance.main: Creation complete after 45s [id=i-...]"
    static ref COMPLETE_RE: Regex =
        Regex::new(r"^\S+: (?:Creation|Modification|Destruction) complete").unwrap();

    // Apply summary: "Apply complete! Resources: ..."
    static ref APPLY_SUMMARY_RE: Regex =
        Regex::new(r"^Apply complete!").unwrap();

    // Changes to Outputs section
    static ref OUTPUTS_HEADER_RE: Regex =
        Regex::new(r"^(?:Changes to )?Outputs:").unwrap();

    // Output value lines: "+ key = value" or "key = value"
    static ref OUTPUT_VALUE_RE: Regex =
        Regex::new(r#"^\s*[+~]?\s*\w[\w_]* = "#).unwrap();
}

/// Filter `terraform plan` output.
///
/// Strips:
/// - "Refreshing state..." lines
/// - Attribute lines with "(known after apply)"
/// - Block close braces of resource blocks
/// - Tag sub-block lines
/// - Verbose preamble text
///
/// Keeps:
/// - Resource header comments ("# resource will be created")
/// - Resource block opener ("+ resource ... {")
/// - Important attributes (ami, instance_type, key_name, name, vpc_id, subnet_id,
///   description, and non-KAA attributes)
/// - Plan summary line
///
/// Target: >=80% token savings.
pub fn filter_terraform_plan(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut kept: Vec<&str> = Vec::new();
    let mut in_resource_block = false;
    let mut in_tags_block = false;
    let mut brace_depth: i32 = 0;

    for line in input.lines() {
        // Always keep the plan summary line
        if PLAN_SUMMARY_RE.is_match(line) {
            kept.push(line);
            continue;
        }

        // Strip refreshing state lines
        if REFRESH_RE.is_match(line) {
            continue;
        }

        // Resource header comment line: "  # aws_instance.main will be created"
        if RESOURCE_HEADER_RE.is_match(line) {
            in_resource_block = true;
            in_tags_block = false;
            brace_depth = 0;
            kept.push(line);
            continue;
        }

        // Resource block opener
        if RESOURCE_BLOCK_OPEN_RE.is_match(line) {
            kept.push(line);
            brace_depth = 1;
            continue;
        }

        if !in_resource_block {
            continue;
        }

        // Track brace depth changes
        let open_count = line.chars().filter(|&c| c == '{').count() as i32;
        let close_count = line.chars().filter(|&c| c == '}').count() as i32;

        // Tags sub-block handling
        if TAGS_OPEN_RE.is_match(line) {
            in_tags_block = true;
            brace_depth += open_count - close_count;
            continue;
        }

        if in_tags_block {
            brace_depth += open_count - close_count;
            if brace_depth <= 1 {
                in_tags_block = false;
            }
            continue;
        }

        // Closing brace of the resource block
        if BLOCK_CLOSE_RE.is_match(line) {
            brace_depth += open_count - close_count;
            if brace_depth <= 0 {
                in_resource_block = false;
                brace_depth = 0;
            }
            continue;
        }

        // Skip known-after-apply attribute lines
        if KNOWN_AFTER_APPLY_RE.is_match(line) {
            brace_depth += open_count - close_count;
            continue;
        }

        // Skip blank lines inside blocks
        if line.trim().is_empty() {
            continue;
        }

        brace_depth += open_count - close_count;
        kept.push(line);
    }

    if kept.is_empty() {
        return input.to_string();
    }

    kept.join("\n")
}

/// Filter `terraform apply` output.
///
/// Keeps:
/// - "Creating..." / "Modifying..." / "Destroying..." start lines
/// - "Creation/Modification/Destruction complete" lines
/// - "Apply complete! Resources: ..." summary
///
/// Strips:
/// - "Still creating... [Ns elapsed]" progress lines
/// - Blank lines
///
/// Target: >=75% token savings.
pub fn filter_terraform_apply(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut kept: Vec<&str> = Vec::new();

    for line in input.lines() {
        // Always keep action start lines
        if ACTION_START_RE.is_match(line) {
            kept.push(line);
            continue;
        }
        // Always keep completion lines
        if COMPLETE_RE.is_match(line) {
            kept.push(line);
            continue;
        }
        // Always keep apply summary
        if APPLY_SUMMARY_RE.is_match(line) {
            kept.push(line);
            continue;
        }
        // Keep Outputs section header and values
        if OUTPUTS_HEADER_RE.is_match(line) {
            kept.push(line);
            continue;
        }
        if OUTPUT_VALUE_RE.is_match(line) {
            kept.push(line);
            continue;
        }
        // Strip progress dots and blank lines (implicit by not pushing)
    }

    if kept.is_empty() {
        return input.to_string();
    }

    kept.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    // ── terraform plan ───────────────────────────────────────────────────────

    #[test]
    fn test_terraform_plan_snapshot() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_plan_raw.txt");
        let output = filter_terraform_plan(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_terraform_plan_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_plan_raw.txt");
        let output = filter_terraform_plan(input);
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
    fn test_terraform_plan_empty() {
        assert_eq!(filter_terraform_plan(""), "");
    }

    #[test]
    fn test_terraform_plan_malformed() {
        let output = filter_terraform_plan("not terraform output\nrandom text here");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_terraform_plan_keeps_summary() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_plan_raw.txt");
        let output = filter_terraform_plan(input);
        assert!(
            output.contains("Plan: 2 to add"),
            "Output should contain plan summary line"
        );
    }

    #[test]
    fn test_terraform_plan_strips_refresh_lines() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_plan_raw.txt");
        let output = filter_terraform_plan(input);
        assert!(
            !output.contains("Refreshing state"),
            "Output should strip 'Refreshing state' lines"
        );
    }

    #[test]
    fn test_terraform_plan_strips_known_after_apply() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_plan_raw.txt");
        let output = filter_terraform_plan(input);
        assert!(
            !output.contains("(known after apply)"),
            "Output should strip '(known after apply)' attribute lines"
        );
    }

    #[test]
    fn test_terraform_plan_keeps_resource_headers() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_plan_raw.txt");
        let output = filter_terraform_plan(input);
        assert!(
            output.contains("aws_instance.main"),
            "Output should include resource name aws_instance.main"
        );
        assert!(
            output.contains("aws_security_group.main"),
            "Output should include resource name aws_security_group.main"
        );
    }

    // ── terraform apply ──────────────────────────────────────────────────────

    #[test]
    fn test_terraform_apply_snapshot() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_apply_raw.txt");
        let output = filter_terraform_apply(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_terraform_apply_savings() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_apply_raw.txt");
        let output = filter_terraform_apply(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 75.0,
            "Expected >=75% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_terraform_apply_empty() {
        assert_eq!(filter_terraform_apply(""), "");
    }

    #[test]
    fn test_terraform_apply_malformed() {
        let output = filter_terraform_apply("not terraform output\nrandom text here");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_terraform_apply_keeps_summary() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_apply_raw.txt");
        let output = filter_terraform_apply(input);
        assert!(
            output.contains("Apply complete!"),
            "Output should contain 'Apply complete!' summary"
        );
        assert!(
            output.contains("2 added"),
            "Output should contain resource count"
        );
    }

    #[test]
    fn test_terraform_apply_strips_progress_dots() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_apply_raw.txt");
        let output = filter_terraform_apply(input);
        assert!(
            !output.contains("Still creating"),
            "Output should strip 'Still creating' progress lines"
        );
    }

    #[test]
    fn test_terraform_apply_keeps_completion_lines() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_apply_raw.txt");
        let output = filter_terraform_apply(input);
        assert!(
            output.contains("Creation complete"),
            "Output should keep 'Creation complete' lines"
        );
    }

    #[test]
    fn test_terraform_apply_keeps_both_resources() {
        let input = include_str!("../../../tests/fixtures/cloud/terraform_apply_raw.txt");
        let output = filter_terraform_apply(input);
        assert!(
            output.contains("aws_instance.main"),
            "Output should mention aws_instance.main"
        );
        assert!(
            output.contains("aws_security_group.main"),
            "Output should mention aws_security_group.main"
        );
    }
}
