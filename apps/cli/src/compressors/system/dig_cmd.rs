use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref ANSWER_HEADER_RE: Regex = Regex::new(r"^;; ANSWER SECTION:").unwrap();
    static ref SECTION_HEADER_RE: Regex = Regex::new(r"^;;\s+\w").unwrap();
    static ref COMMENT_RE: Regex = Regex::new(r"^\s*;").unwrap();
}

/// Filter `dig` output: keep only the ANSWER section records.
/// Drops headers, query metadata, AUTHORITY/ADDITIONAL/OPT/QUESTION sections.
pub fn filter_dig(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut answers: Vec<&str> = Vec::new();
    let mut in_answer = false;

    for line in input.lines() {
        if ANSWER_HEADER_RE.is_match(line) {
            in_answer = true;
            continue;
        }
        if in_answer {
            // Section ends at next ";;" header or blank line
            if line.trim().is_empty() || SECTION_HEADER_RE.is_match(line) {
                in_answer = false;
                continue;
            }
            if !COMMENT_RE.is_match(line) {
                answers.push(line);
            }
        }
    }

    if answers.is_empty() {
        // Fallback: collect bare resource records (in case of `dig +short` or
        // `dig +noall +answer` style output where there's no ANSWER header).
        let bare: Vec<&str> = input
            .lines()
            .filter(|l| !COMMENT_RE.is_match(l) && !l.trim().is_empty())
            .collect();
        if bare.is_empty() {
            return input.to_string();
        }
        return bare.join("\n");
    }

    answers.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_dig_format() {
        let input = include_str!("../../../tests/fixtures/system/dig_raw.txt");
        let output = filter_dig(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_dig_savings() {
        let input = include_str!("../../../tests/fixtures/system/dig_raw.txt");
        let output = filter_dig(input);
        let input_t = count_tokens(input);
        let output_t = count_tokens(&output);
        let savings = 100.0 - (output_t as f64 / input_t as f64 * 100.0);
        assert!(
            savings >= 60.0,
            "Expected >=60% savings, got {:.1}% ({} -> {} tokens)",
            savings,
            input_t,
            output_t
        );
    }

    #[test]
    fn test_dig_empty() {
        assert_eq!(filter_dig(""), "");
    }

    #[test]
    fn test_dig_keeps_answer_record() {
        let input = include_str!("../../../tests/fixtures/system/dig_raw.txt");
        let output = filter_dig(input);
        assert!(output.contains("google.com"));
        assert!(output.contains("172.217.16.142"));
    }

    #[test]
    fn test_dig_drops_metadata() {
        let input = include_str!("../../../tests/fixtures/system/dig_raw.txt");
        let output = filter_dig(input);
        assert!(!output.contains("Query time"));
        assert!(!output.contains("MSG SIZE"));
        assert!(!output.contains("HEADER"));
        assert!(!output.contains("PSEUDOSECTION"));
    }

    #[test]
    fn test_dig_short_form_passthrough() {
        // `dig +short google.com` produces just IP addresses, no headers
        let input = "172.217.20.78\n142.250.190.78\n";
        let output = filter_dig(input);
        assert!(output.contains("172.217.20.78"));
        assert!(output.contains("142.250.190.78"));
    }

    #[test]
    fn test_dig_malformed_passthrough() {
        let output = filter_dig("not dig output");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_dig_unicode() {
        let input =
            "; comment with ünïcödé\n;; ANSWER SECTION:\nexämple.com.   60   IN   A   1.2.3.4\n";
        let output = filter_dig(input);
        assert!(output.contains("exämple.com"));
    }

    #[test]
    fn test_dig_multiple_answers() {
        let input = ";; ANSWER SECTION:\ngoogle.com. 60 IN A 1.2.3.4\ngoogle.com. 60 IN A 5.6.7.8\n\n;; AUTHORITY SECTION:\nignore me\n";
        let output = filter_dig(input);
        assert!(output.contains("1.2.3.4"));
        assert!(output.contains("5.6.7.8"));
        assert!(!output.contains("ignore me"));
    }
}
