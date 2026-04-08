#[allow(dead_code)]
/// Count whitespace-delimited tokens (proxy for LLM token count).
pub fn count_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

#[allow(dead_code)]
/// Assert token savings meet minimum threshold.
pub fn assert_savings(input: &str, output: &str, min_pct: f64, label: &str) {
    let input_tokens = count_tokens(input);
    let output_tokens = count_tokens(output);
    if input_tokens == 0 {
        return;
    }
    let savings = 100.0 - (output_tokens as f64 / input_tokens as f64 * 100.0);
    assert!(
        savings >= min_pct,
        "{}: Expected >={:.0}% savings, got {:.1}% ({} -> {} tokens)",
        label,
        min_pct,
        savings,
        input_tokens,
        output_tokens
    );
}

#[allow(dead_code)]
/// Standard edge-case battery for any compressor filter function.
/// Verifies the filter doesn't panic on: empty, unicode, ANSI, long lines,
/// many lines, and binary-like content.
pub fn run_edge_cases<F: Fn(&str) -> String>(filter: F, name: &str) {
    // Empty input
    let _empty = filter("");

    // Unicode
    let _unicode = filter("日本語テキスト\nαβγδ\n🎉 emoji line");

    // ANSI escape codes
    let _ansi = filter("\x1b[32mgreen\x1b[0m normal \x1b[1mbold\x1b[0m");

    // Very long single line (10KB)
    let long_line = "x".repeat(10_000);
    let _long = filter(&long_line);

    // Many short lines (1000)
    let many_lines: String = (0..1000)
        .map(|i| format!("line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    let _many = filter(&many_lines);

    // Binary-like content
    let _binary = filter("ELF\x00\x01\x02 header \x00 content");

    // Only whitespace
    let _whitespace = filter("   \n\t\n   ");

    // Repeated identical lines
    let repeated = "same line\n".repeat(500);
    let _repeated = filter(&repeated);

    let _ = name; // used for future error messages
}
