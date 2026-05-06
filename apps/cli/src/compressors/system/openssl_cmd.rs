use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Hex octet line: `aa:bb:cc:dd:...` (the body of pub keys, signatures, SCT timestamps).
    /// Two or more colon-separated hex pairs, with optional trailing colon (continuation).
    static ref HEX_OCTETS_RE: Regex =
        Regex::new(r"^\s*[0-9A-Fa-f]{2}(?::[0-9A-Fa-f]{2}){1,}:?\s*$").unwrap();
}

/// Headers that introduce a block we want to drop entirely (header + every
/// line indented more than it). Their bodies are either hex (already covered)
/// or low-signal metadata (URLs, OIDs, key IDs).
const DROP_BLOCK_HEADERS: &[&str] = &[
    "Serial Number:",
    "X509v3 Subject Key Identifier:",
    "X509v3 Authority Key Identifier:",
    "Authority Information Access:",
    "X509v3 Certificate Policies:",
    "CT Precertificate SCTs:",
];

/// Solo lines to drop without dropping anything below them. Mostly key-info
/// scaffolding that adds nothing once the hex pub-key bytes are stripped.
const DROP_SOLO: &[&str] = &[
    "Subject Public Key Info:",
    "pub:",
    "ASN1 OID:",
    "NIST CURVE:",
];

/// Filter `openssl x509 -text` output: keep subject/issuer/validity/SAN/key
/// info, drop verbose hex octet blocks (pub key bytes, signatures, SCT
/// timestamps), key identifier hashes, AIA URLs, certificate policies, and
/// the duplicated trailer signature.
pub fn filter_openssl(input: &str) -> String {
    if input.trim().is_empty() {
        return String::new();
    }

    let mut kept: Vec<&str> = Vec::new();
    let mut block_indent: Option<usize> = None;
    let mut after_sig_value = false;

    for line in input.lines() {
        if after_sig_value {
            continue;
        }

        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // The trailer block at the bottom: drop "Signature Value:" and all
        // following lines (signature hex). The matching "Signature Algorithm"
        // at indent 4 (a duplicate of the cert's sig alg in the Data section)
        // also goes here.
        if trimmed.starts_with("Signature Value:") {
            after_sig_value = true;
            continue;
        }
        if indent == 4 && trimmed.starts_with("Signature Algorithm:") {
            continue;
        }

        // Inside a drop-block: skip until we exit (next line at <= block_indent).
        if let Some(bi) = block_indent {
            if !trimmed.is_empty() && indent <= bi {
                block_indent = None;
            } else {
                continue;
            }
        }

        // Entering a new drop-block?
        if DROP_BLOCK_HEADERS.iter().any(|h| trimmed.starts_with(h)) {
            block_indent = Some(indent);
            continue;
        }

        // Solo drops (no block).
        if DROP_SOLO.iter().any(|s| trimmed.starts_with(s)) {
            continue;
        }

        // Hex byte rows.
        if HEX_OCTETS_RE.is_match(line) {
            continue;
        }

        kept.push(line);
    }

    if kept.is_empty() || kept.iter().all(|l| l.trim().is_empty()) {
        return input.to_string();
    }

    kept.join("\n").trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn test_openssl_x509_format() {
        let input = include_str!("../../../tests/fixtures/system/openssl_x509_raw.txt");
        let output = filter_openssl(input);
        assert_snapshot!(output);
    }

    #[test]
    fn test_openssl_x509_savings() {
        let input = include_str!("../../../tests/fixtures/system/openssl_x509_raw.txt");
        let output = filter_openssl(input);
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
    fn test_openssl_empty() {
        assert_eq!(filter_openssl(""), "");
    }

    #[test]
    fn test_openssl_keeps_subject_issuer_validity() {
        let input = include_str!("../../../tests/fixtures/system/openssl_x509_raw.txt");
        let output = filter_openssl(input);
        assert!(output.contains("Subject: CN=github.com"));
        assert!(output.contains("Issuer:"));
        assert!(output.contains("Not Before:"));
        assert!(output.contains("Not After"));
    }

    #[test]
    fn test_openssl_keeps_san() {
        let input = include_str!("../../../tests/fixtures/system/openssl_x509_raw.txt");
        let output = filter_openssl(input);
        assert!(output.contains("Subject Alternative Name"));
        assert!(output.contains("DNS:github.com"));
    }

    #[test]
    fn test_openssl_drops_pubkey_bytes() {
        let input = include_str!("../../../tests/fixtures/system/openssl_x509_raw.txt");
        let output = filter_openssl(input);
        // Sample hex bytes from the fixture's public key
        assert!(!output.contains("04:bd:d3:72:a4:47"));
        assert!(!output.contains("7a:6f:dd:11:fd"));
    }

    #[test]
    fn test_openssl_drops_signature_bytes() {
        let input = include_str!("../../../tests/fixtures/system/openssl_x509_raw.txt");
        let output = filter_openssl(input);
        // Sample hex bytes from the fixture's signature value
        assert!(!output.contains("30:45:02:20:11:7e"));
    }

    #[test]
    fn test_openssl_malformed_passthrough() {
        let output = filter_openssl("not openssl output\nrandom text\n");
        assert!(!output.is_empty());
    }

    #[test]
    fn test_openssl_unicode_subject() {
        let input = "Certificate:\n    Data:\n        Subject: CN=exämple.com\n";
        let output = filter_openssl(input);
        assert!(output.contains("exämple.com"));
    }
}
