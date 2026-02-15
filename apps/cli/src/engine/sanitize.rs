use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;

lazy_static! {
    /// Match home directory paths like /Users/username/ or /home/username/
    static ref HOME_PATH_RE: Regex = Regex::new(r"(/Users/|/home/)[a-zA-Z0-9_.\-]+").unwrap();
    /// Match username@ patterns (e.g., user@host)
    static ref USER_AT_RE: Regex = Regex::new(r"[a-zA-Z0-9_.\-]+@[a-zA-Z0-9_.\-]+").unwrap();
    /// Match common secret patterns
    static ref SECRET_RE: Regex = Regex::new(
        r"(?i)(api[_\-]?key|token|secret|password|credential|auth)\s*[:=]\s*\S+"
    ).unwrap();
    /// Match Bearer tokens
    static ref BEARER_RE: Regex = Regex::new(r"Bearer\s+[A-Za-z0-9\-._~+/]+=*").unwrap();
    /// Match GitHub PATs
    static ref GHP_RE: Regex = Regex::new(r"ghp_[A-Za-z0-9]{36}").unwrap();
    /// Match AWS access key IDs
    static ref AWS_KEY_RE: Regex = Regex::new(r"AKIA[0-9A-Z]{16}").unwrap();
    /// Match OpenAI API keys
    static ref OPENAI_RE: Regex = Regex::new(r"sk-[A-Za-z0-9]{48}").unwrap();
    /// Match private key headers
    static ref PRIVKEY_RE: Regex = Regex::new(r"(?i)-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----").unwrap();
}

/// Redact home directory paths: /Users/theo/ -> /Users/user/
pub fn redact_paths(input: &str) -> Cow<'_, str> {
    if HOME_PATH_RE.is_match(input) {
        Cow::Owned(HOME_PATH_RE.replace_all(input, "${1}user").to_string())
    } else {
        Cow::Borrowed(input)
    }
}

/// Redact secrets (API keys, tokens, passwords) from output.
pub fn redact_secrets(input: &str) -> Cow<'_, str> {
    let mut result = input.to_string();
    let mut changed = false;

    for re in &[
        &*SECRET_RE,
        &*BEARER_RE,
        &*GHP_RE,
        &*AWS_KEY_RE,
        &*OPENAI_RE,
        &*PRIVKEY_RE,
    ] {
        if re.is_match(&result) {
            result = re.replace_all(&result, "[REDACTED]").to_string();
            changed = true;
        }
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(input)
    }
}

/// Full sanitization pipeline: paths + secrets.
pub fn sanitize_output(input: &str) -> String {
    let step1 = redact_paths(input);
    let step2 = redact_secrets(&step1);
    step2.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_home_paths() {
        let input = "drwxr-xr-x  2 user staff  64 /Users/johndoe/projects";
        let output = redact_paths(input);
        assert!(!output.contains("johndoe"));
        assert!(output.contains("/Users/user"));
    }

    #[test]
    fn test_redact_linux_paths() {
        let input = "/home/devuser/.config/tok0";
        let output = redact_paths(input);
        assert!(!output.contains("devuser"));
        assert!(output.contains("/home/user"));
    }

    #[test]
    fn test_no_paths_zero_copy() {
        let input = "no paths here";
        let output = redact_paths(input);
        assert!(matches!(output, Cow::Borrowed(_)));
    }

    #[test]
    fn test_redact_api_key() {
        let input = "API_KEY=sk-abc123def456";
        let output = redact_secrets(input);
        assert!(output.contains("[REDACTED]"));
        assert!(!output.contains("sk-abc123"));
    }

    #[test]
    fn test_redact_bearer_token() {
        let input = "Authorization: Bearer eyJhbGciOiJSUzI1NiJ9.payload.sig";
        let output = redact_secrets(input);
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_github_pat() {
        let input = "token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
        let output = redact_secrets(input);
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_aws_key() {
        let input = "aws_access_key_id = AKIAIOSFODNN7EXAMPLE";
        let output = redact_secrets(input);
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_private_key() {
        let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAK...";
        let output = redact_secrets(input);
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn test_no_secrets_zero_copy() {
        let input = "just normal output";
        let output = redact_secrets(input);
        assert!(matches!(output, Cow::Borrowed(_)));
    }

    #[test]
    fn test_sanitize_combined() {
        let input = "PATH=/Users/johndoe/bin\nAPI_KEY=secret123";
        let output = sanitize_output(input);
        assert!(!output.contains("johndoe"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn test_password_in_env() {
        let input = "DATABASE_PASSWORD=mysecretpass123";
        let output = redact_secrets(input);
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn test_token_in_url() {
        let input = "auth_token: abc123xyz789";
        let output = redact_secrets(input);
        assert!(output.contains("[REDACTED]"));
    }
}
