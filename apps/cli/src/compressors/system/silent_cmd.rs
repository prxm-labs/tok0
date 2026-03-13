/// Filter for silent commands that produce no stdout on success.
///
/// Commands: `touch`, `mkdir`, `cp`, `mv`, `rm`, `chmod`, `chown`, `ln`
/// If stdout is empty or whitespace-only → return "ok".
/// Otherwise passthrough unchanged.
pub fn filter_silent(input: &str) -> String {
    if input.trim().is_empty() {
        "ok".to_string()
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_empty_input_returns_ok() {
        assert_eq!(filter_silent(""), "ok");
    }

    #[test]
    fn test_whitespace_only_returns_ok() {
        assert_eq!(filter_silent("   \n\n  \t  "), "ok");
    }

    #[test]
    fn test_newlines_only_returns_ok() {
        assert_eq!(filter_silent("\n\n\n"), "ok");
    }

    #[test]
    fn test_nonempty_passthrough() {
        let input = "cp: cannot stat 'foo': No such file or directory";
        assert_eq!(filter_silent(input), input);
    }

    #[test]
    fn test_error_output_passthrough() {
        let input = "mkdir: cannot create directory 'test': File exists";
        assert_eq!(filter_silent(input), input);
    }

    #[test]
    fn test_savings_on_empty() {
        let input = "";
        let output = filter_silent(input);
        // Empty input produces "ok" — 2 chars vs 0, but the point
        // is we replace a no-information empty string with a clear signal
        assert_eq!(output, "ok");
        assert_eq!(count_tokens(&output), 1);
    }

    #[test]
    fn test_verbose_output_passthrough() {
        let input = "removed 'file1.txt'\nremoved 'file2.txt'\nremoved 'file3.txt'";
        let output = filter_silent(input);
        assert_eq!(
            output, input,
            "Non-empty output should pass through unchanged"
        );
    }
}
