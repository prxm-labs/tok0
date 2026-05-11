use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;

lazy_static! {
    /// Session-id sanitization: only allow URL-safe identifier chars so we
    /// can safely inline `TOK0_SESSION_ID=<id>` into the shell command without
    /// quoting. Anything else is dropped (env var omitted).
    static ref SAFE_SESSION_ID: Regex = Regex::new(r"^[A-Za-z0-9_\-]+$").unwrap();
}

lazy_static! {
    /// Map of command names to tok0 subcommands. Commands not in this map
    /// pass through with `tok0` prepended.
    static ref REWRITE_MAP: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        // File reading variants -> tok0 read
        m.insert("cat",  "read");
        m.insert("head", "read");
        m.insert("tail", "read");
        // Directory listing
        m.insert("tree", "ls --tree");
        // Search
        m.insert("rg",   "grep");
        m
    };
}

/// Sanitize a tool id: allow only `[a-z_]` (canonical form is snake_case
/// like `claude_code`). Anything else returns `None` so the env var is
/// omitted — never trust upstream identifiers verbatim in a shell context.
fn sanitize_tool(tool: &str) -> Option<&str> {
    if tool.is_empty() || !tool.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
        return None;
    }
    Some(tool)
}

/// Build the inline-env prefix for a rewritten command. PreToolUse hooks
/// substitute the command string and the substituted command runs in a
/// fresh shell, so env vars exported in the hook script don't reach the
/// rewritten `tok0` invocation. Inlining `KEY=val ` at the front is the
/// only carrier that survives.
///
/// Returns an empty string if `tool` is missing or invalid.
fn env_prefix(tool: Option<&str>, session_id: Option<&str>) -> String {
    let Some(tool_clean) = tool.and_then(sanitize_tool) else {
        return String::new();
    };
    let session_clean = session_id.filter(|s| SAFE_SESSION_ID.is_match(s));
    match session_clean {
        Some(sid) => format!("TOK0_TOOL={tool_clean} TOK0_SESSION_ID={sid} "),
        None => format!("TOK0_TOOL={tool_clean} "),
    }
}

/// A segment of a compound shell command — either a command text or a
/// top-level operator (`&&`, `||`, `;`). Returned by
/// `split_compound_command`.
#[derive(Debug, PartialEq, Eq)]
enum Segment<'a> {
    Cmd(&'a str),
    Op(&'a str),
}

/// Result of attempting to split a compound command.
#[derive(Debug)]
enum SplitResult<'a> {
    /// No top-level compound operators were found. Caller should fall
    /// through to the single-command rewrite path.
    NoCompound,
    /// A trigger that makes rewriting unsafe was detected (heredoc,
    /// pipe, command substitution, unbalanced quote, empty segment).
    /// Caller should return the input verbatim.
    Passthrough,
    /// Successfully split into alternating Cmd/Op segments.
    Compound(Vec<Segment<'a>>),
}

/// Split a command on top-level `&&`, `||`, `;` while respecting quotes.
/// See `SplitResult` for the three outcomes.
fn split_compound_command(cmd: &str) -> SplitResult<'_> {
    if cmd.contains("<<") {
        return SplitResult::Passthrough;
    }
    let bytes = cmd.as_bytes();
    let mut segments: Vec<Segment<'_>> = Vec::new();
    let mut last = 0usize;
    let mut i = 0usize;
    // Quote state: ' ', '\'', '"'  (space = not in quotes).
    let mut quote: u8 = b' ';
    while i < bytes.len() {
        let b = bytes[i];
        // Handle quote state first.
        match (quote, b) {
            (b' ', b'\'') => {
                quote = b'\'';
                i += 1;
                continue;
            }
            (b' ', b'"') => {
                quote = b'"';
                i += 1;
                continue;
            }
            (b'\'', b'\'') => {
                quote = b' ';
                i += 1;
                continue;
            }
            (b'"', b'"') => {
                quote = b' ';
                i += 1;
                continue;
            }
            (b'"', b'\\') => {
                // Skip escaped char inside double quotes.
                i += 2;
                continue;
            }
            _ => {}
        }
        if quote != b' ' {
            i += 1;
            continue;
        }
        // At top level — check for the bail-out triggers.
        if b == b'|' && bytes.get(i + 1).copied() != Some(b'|') {
            return SplitResult::Passthrough; // single pipe
        }
        if b == b'`' {
            return SplitResult::Passthrough; // backtick command substitution
        }
        if b == b'$' && bytes.get(i + 1).copied() == Some(b'(') {
            return SplitResult::Passthrough; // $( command substitution
        }
        // Top-level escape (outside quotes) — skip next char.
        if b == b'\\' {
            i += 2;
            continue;
        }
        // Match operators in order of length: longest first.
        let next_two = bytes.get(i..i + 2);
        let op_len: Option<usize> = if next_two == Some(b"&&") || next_two == Some(b"||") {
            Some(2)
        } else if b == b';' {
            Some(1)
        } else {
            None
        };
        if let Some(len) = op_len {
            let seg = cmd[last..i].trim_matches(' ');
            if seg.is_empty() {
                // Leading/trailing operator or `;;` doubled — too weird.
                return SplitResult::Passthrough;
            }
            segments.push(Segment::Cmd(seg));
            segments.push(Segment::Op(&cmd[i..i + len]));
            i += len;
            last = i;
            continue;
        }
        i += 1;
    }
    if quote != b' ' {
        return SplitResult::Passthrough; // unbalanced quote
    }
    let tail = cmd[last..].trim_matches(' ');
    if segments.is_empty() {
        return SplitResult::NoCompound;
    }
    if tail.is_empty() {
        return SplitResult::Passthrough; // trailing operator
    }
    segments.push(Segment::Cmd(tail));
    SplitResult::Compound(segments)
}

/// Rewrite a single-command segment (no compound operators expected).
/// Used as the per-segment building block by `rewrite_bash_command` after
/// it splits compound input.
fn rewrite_single_segment(cmd: &str) -> String {
    let trimmed = cmd.trim_start();
    if trimmed.is_empty() {
        return cmd.to_string();
    }
    if trimmed.starts_with("tok0 ") || trimmed == "tok0" {
        return cmd.to_string();
    }
    if cmd.contains("<<") {
        return cmd.to_string();
    }
    let first = trimmed.split_whitespace().next().unwrap_or("");
    if crate::engine::guards::is_shell_builtin(first)
        || crate::engine::guards::is_var_assignment(first)
    {
        return cmd.to_string();
    }
    if let Some(&mapped) = REWRITE_MAP.get(first) {
        let rest = trimmed[first.len()..].trim_start();
        if rest.is_empty() {
            format!("tok0 {}", mapped)
        } else {
            format!("tok0 {} {}", mapped, rest)
        }
    } else {
        format!("tok0 {}", trimmed)
    }
}

/// Rewrite a single bash command string (as delivered by Claude Code's
/// PreToolUse hook on stdin) so the resulting command runs through tok0.
///
/// Compound commands joined by `&&`, `||`, `;` are split at top level
/// (respecting quotes) and each segment is rewritten independently:
/// `cargo test && cargo build` → `tok0 cargo test && tok0 cargo build`.
///
/// Passthrough cases — leave unchanged:
/// - Empty / whitespace-only input
/// - Pipes (`|`) — deliberate punt; mixing with `should_compress` is
///   subtle enough to warrant a follow-up
/// - Heredocs (`<<`) — would corrupt the heredoc body
/// - Command substitution (`$(…)`, backticks) — too complex to recurse
///   into a subshell
/// - Unbalanced quotes — can't tokenize safely
///
/// For each rewriteable segment, applies `REWRITE_MAP` (rg→grep,
/// cat→read, etc.) and skips shell builtins / variable-assignment
/// prefixes.
#[allow(dead_code)] // used by tests + retained as a public API; main.rs calls the _with_env variant
pub fn rewrite_bash_command(cmd: &str) -> String {
    if cmd.trim().is_empty() {
        return cmd.to_string();
    }
    let segments = match split_compound_command(cmd) {
        SplitResult::NoCompound => return rewrite_single_segment(cmd),
        SplitResult::Passthrough => return cmd.to_string(),
        SplitResult::Compound(segs) => segs,
    };
    emit_compound(cmd, &segments, "")
}

/// Join the split segments back into a compound command string. `prefix`
/// is prepended to each Cmd segment that actually got rewritten (so
/// passthrough segments — builtins, var-assignments — keep their shape).
fn emit_compound(original: &str, segments: &[Segment<'_>], prefix: &str) -> String {
    let mut out = String::with_capacity(original.len() + segments.len() * (8 + prefix.len()));
    for (idx, seg) in segments.iter().enumerate() {
        match seg {
            Segment::Cmd(c) => {
                let rewritten = rewrite_single_segment(c);
                let was_rewritten = rewritten != *c && rewritten.starts_with("tok0");
                if was_rewritten && !prefix.is_empty() {
                    out.push_str(prefix);
                }
                out.push_str(&rewritten);
            }
            Segment::Op(op) => {
                // ';' typically hugs the preceding token; '&&'/'||' get
                // spaces on both sides. Match common shell formatting.
                if *op == ";" {
                    out.push_str("; ");
                } else {
                    out.push(' ');
                    out.push_str(op);
                    out.push(' ');
                }
            }
        }
        // Defensive: should never end with an Op (split rejects trailing).
        let _ = idx;
    }
    out
}

/// Like `rewrite_bash_command`, but each rewritten segment is prefixed
/// with `TOK0_TOOL=<tool> TOK0_SESSION_ID=<id> ` so the downstream tok0
/// process can dispatch via `tool_policy::current()` and scope per-session
/// state. For compound commands, every rewritten segment gets its own
/// prefix; passthrough segments (builtins, var-assignments) keep their
/// shape.
///
/// Passthrough commands (heredocs, pipes, command substitution,
/// unbalanced quotes) are returned verbatim — adding env to them would
/// change shell parse semantics and defeats the safety reason for
/// passthrough.
pub fn rewrite_bash_command_with_env(
    cmd: &str,
    tool: Option<&str>,
    session_id: Option<&str>,
) -> String {
    if cmd.trim().is_empty() {
        return cmd.to_string();
    }
    let prefix = env_prefix(tool, session_id);
    let segments = match split_compound_command(cmd) {
        SplitResult::Passthrough => return cmd.to_string(),
        SplitResult::NoCompound => {
            let rewritten = rewrite_single_segment(cmd);
            if rewritten == cmd || !rewritten.starts_with("tok0") || prefix.is_empty() {
                return rewritten;
            }
            return format!("{prefix}{rewritten}");
        }
        SplitResult::Compound(segs) => segs,
    };
    emit_compound(cmd, &segments, &prefix)
}

/// Rewrite a command so it runs through tok0.
///
/// Rules:
/// - Empty `args` → returns empty string.
/// - Already starts with `tok0` → return as-is (no double-wrap).
/// - `args[0]` is in `REWRITE_MAP` → `tok0 <mapped-subcommand> <rest>`.
/// - Contains heredoc markers (`<<`) → pass through unchanged.
/// - Anything else → `tok0 <args>`.
pub fn rewrite_command(args: &[String]) -> String {
    if args.is_empty() {
        return String::new();
    }

    // No double-rewrite
    if args[0] == "tok0" {
        return args.join(" ");
    }

    // Heredoc passthrough: if any arg contains `<<`, return as-is
    if args.iter().any(|a| a.contains("<<")) {
        return args.join(" ");
    }

    // Shell-metachar passthrough. If args contain a standalone shell
    // operator (`&&`, `||`, `;`, `|`, `>`, `<`, `&`), the caller most
    // likely embedded a compound command — wrapping the whole join in
    // `tok0 ` would change shell parse semantics. Leave it alone.
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "&&" | "||" | ";" | "|" | ">" | "<" | "&"))
    {
        return args.join(" ");
    }

    let cmd = args[0].as_str();

    // Guards that would fail at the External arm anyway — pass through
    // so the caller (typically a test or manual invocation) sees the
    // original input rather than a tok0-prefixed broken command.
    if crate::engine::guards::is_shell_builtin(cmd)
        || crate::engine::guards::is_var_assignment(cmd)
        || crate::engine::guards::needs_tty(cmd)
    {
        return args.join(" ");
    }

    if let Some(&mapped) = REWRITE_MAP.get(cmd) {
        // The rest of the original args (args[1..])
        let rest = args[1..].join(" ");
        if rest.is_empty() {
            format!("tok0 {}", mapped)
        } else {
            format!("tok0 {} {}", mapped, rest)
        }
    } else {
        format!("tok0 {}", args.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|&x| x.to_string()).collect()
    }

    // ─────────────────────────────────────────────────────────────────
    // rewrite_bash_command — operates on the single bash command string
    // delivered by Claude Code's PreToolUse hook on stdin.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_bash_basic_prefix() {
        assert_eq!(rewrite_bash_command("git status"), "tok0 git status");
        assert_eq!(rewrite_bash_command("cargo build"), "tok0 cargo build");
    }

    #[test]
    fn test_bash_rewrite_map_applied() {
        // `rg` → `tok0 grep`, `cat` → `tok0 read`, `tree` → `tok0 ls --tree`.
        assert_eq!(rewrite_bash_command("rg pattern"), "tok0 grep pattern");
        assert_eq!(
            rewrite_bash_command("cat /etc/hosts"),
            "tok0 read /etc/hosts"
        );
        assert_eq!(rewrite_bash_command("tree src/"), "tok0 ls --tree src/");
    }

    #[test]
    fn test_bash_no_double_wrap() {
        assert_eq!(rewrite_bash_command("tok0 git status"), "tok0 git status");
        assert_eq!(rewrite_bash_command("tok0"), "tok0");
    }

    #[test]
    fn test_bash_builtin_unchanged() {
        // Builtins pass through so the bash tool runs them natively.
        for cmd in [
            "cd /tmp",
            "pushd /tmp",
            "export FOO=bar",
            "source .env",
            "alias ll=ls",
        ] {
            assert_eq!(rewrite_bash_command(cmd), cmd, "{cmd} should pass through");
        }
    }

    #[test]
    fn test_bash_var_assignment_unchanged() {
        assert_eq!(
            rewrite_bash_command("FOO=bar cargo build"),
            "FOO=bar cargo build"
        );
    }

    #[test]
    fn test_bash_pipe_still_passthrough() {
        // | is still passthrough — Phase 6 deliberately doesn't touch pipes
        // because of TOK0_FORCE_COMPRESS interactions with should_compress().
        assert_eq!(
            rewrite_bash_command("git log | head -5"),
            "git log | head -5"
        );
    }

    // ── Phase 6: compound shell rewriter ──────────────────────────────

    #[test]
    fn test_compound_and_rewrites_each_segment() {
        // cargo test && cargo build → each side rewritten independently.
        let out = rewrite_bash_command("cargo test && cargo build");
        assert_eq!(out, "tok0 cargo test && tok0 cargo build");
    }

    #[test]
    fn test_compound_or_rewrites_each_segment() {
        let out = rewrite_bash_command("cargo test || cargo check");
        assert_eq!(out, "tok0 cargo test || tok0 cargo check");
    }

    #[test]
    fn test_compound_semicolon_rewrites_each_segment() {
        let out = rewrite_bash_command("git status; git log");
        assert_eq!(out, "tok0 git status; tok0 git log");
    }

    #[test]
    fn test_compound_mixed_operators() {
        let out = rewrite_bash_command("git pull && cargo test || cargo check");
        assert_eq!(out, "tok0 git pull && tok0 cargo test || tok0 cargo check");
    }

    #[test]
    fn test_compound_builtin_segment_left_alone() {
        // cd is a builtin → not rewritten; the other segment still wrapped.
        let out = rewrite_bash_command("cd /tmp && cargo build");
        assert_eq!(out, "cd /tmp && tok0 cargo build");
    }

    #[test]
    fn test_compound_applies_rewrite_map() {
        // rg → grep; should apply to each chained segment too.
        let out = rewrite_bash_command("rg foo && rg bar");
        assert_eq!(out, "tok0 grep foo && tok0 grep bar");
    }

    #[test]
    fn test_compound_with_already_prefixed_segment() {
        // Idempotency: if a segment is already tok0-prefixed, no double-wrap.
        let out = rewrite_bash_command("tok0 git status && cargo build");
        assert_eq!(out, "tok0 git status && tok0 cargo build");
    }

    #[test]
    fn test_compound_quoted_operator_not_split() {
        // The && is inside a single-quoted string → it's an arg, not an op.
        let out = rewrite_bash_command("echo 'a && b'");
        assert_eq!(out, "tok0 echo 'a && b'");
    }

    #[test]
    fn test_compound_double_quoted_operator_not_split() {
        let out = rewrite_bash_command("echo \"a || b\"");
        assert_eq!(out, "tok0 echo \"a || b\"");
    }

    #[test]
    fn test_compound_pipe_still_passthrough() {
        // | still triggers passthrough — Phase 6 leaves pipes alone.
        let out = rewrite_bash_command("cargo test | grep FAIL");
        assert_eq!(out, "cargo test | grep FAIL");
    }

    #[test]
    fn test_compound_heredoc_still_passthrough() {
        let cmd = "bash <<EOF && ls\nEOF";
        assert_eq!(rewrite_bash_command(cmd), cmd);
    }

    #[test]
    fn test_compound_command_substitution_passthrough() {
        // $( and backticks introduce subshells we don't want to rewrite into.
        let out1 = rewrite_bash_command("echo $(date) && ls");
        assert_eq!(out1, "echo $(date) && ls"); // passthrough on $(
        let out2 = rewrite_bash_command("echo `date` && ls");
        assert_eq!(out2, "echo `date` && ls"); // passthrough on backtick
    }

    #[test]
    fn test_compound_unbalanced_quote_passthrough() {
        // Unbalanced quote → can't safely tokenize → leave alone.
        let cmd = "echo 'foo && ls";
        assert_eq!(rewrite_bash_command(cmd), cmd);
    }

    #[test]
    fn test_compound_var_assignment_segment_preserved() {
        // FOO=bar cargo build → segment is left as-is (var assignment guard)
        let out = rewrite_bash_command("FOO=bar cargo build && cargo test");
        assert_eq!(out, "FOO=bar cargo build && tok0 cargo test");
    }

    #[test]
    fn test_compound_empty_segment_passthrough() {
        // Trailing &&, leading &&, double &&&&, etc. — too weird to rewrite.
        for cmd in ["cargo build &&", "&& cargo build", "cargo a;;cargo b"] {
            assert_eq!(rewrite_bash_command(cmd), cmd, "got: {cmd}");
        }
    }

    #[test]
    fn test_compound_env_prefix_per_segment() {
        // With env: each rewritten segment gets its own env prefix.
        let out = rewrite_bash_command_with_env(
            "cargo test && cargo build",
            Some("claude_code"),
            Some("abc123"),
        );
        assert_eq!(
            out,
            "TOK0_TOOL=claude_code TOK0_SESSION_ID=abc123 tok0 cargo test \
             && TOK0_TOOL=claude_code TOK0_SESSION_ID=abc123 tok0 cargo build"
        );
    }

    #[test]
    fn test_bash_heredoc_unchanged() {
        let cmd = "bash <<EOF\ngit status\nEOF";
        assert_eq!(rewrite_bash_command(cmd), cmd);
    }

    #[test]
    fn test_bash_empty_and_whitespace() {
        assert_eq!(rewrite_bash_command(""), "");
        assert_eq!(rewrite_bash_command("   "), "   ");
    }

    #[test]
    fn test_bash_leading_whitespace_preserved_when_rewriting() {
        // Trim only matters for the predicate check; the resulting
        // command is built from the trimmed form to avoid awkward
        // `tok0    git status` output.
        assert_eq!(rewrite_bash_command("  git status"), "tok0 git status");
    }

    // ── Phase 4: inline-env prefix for per-tool dispatch ──────────────

    #[test]
    fn test_with_env_prepends_tool_only() {
        let out = rewrite_bash_command_with_env("git status", Some("claude_code"), None);
        assert_eq!(out, "TOK0_TOOL=claude_code tok0 git status");
    }

    #[test]
    fn test_with_env_prepends_tool_and_session() {
        let out =
            rewrite_bash_command_with_env("git status", Some("claude_code"), Some("abc123-def456"));
        assert_eq!(
            out,
            "TOK0_TOOL=claude_code TOK0_SESSION_ID=abc123-def456 tok0 git status"
        );
    }

    #[test]
    fn test_with_env_applies_rewrite_map() {
        // rg → grep, env prefix still added
        let out = rewrite_bash_command_with_env("rg foo", Some("gemini"), None);
        assert_eq!(out, "TOK0_TOOL=gemini tok0 grep foo");
    }

    #[test]
    fn test_with_env_compound_each_segment_prefixed() {
        // Phase 6 supersedes Phase 4's passthrough — compound chains are
        // now rewritten per-segment, each with its own env prefix.
        let cmd = "cargo test && cargo build";
        let out = rewrite_bash_command_with_env(cmd, Some("claude_code"), None);
        assert_eq!(
            out,
            "TOK0_TOOL=claude_code tok0 cargo test && TOK0_TOOL=claude_code tok0 cargo build"
        );
    }

    #[test]
    fn test_with_env_passthrough_no_prefix_on_heredoc() {
        let cmd = "bash <<EOF\nls\nEOF";
        let out = rewrite_bash_command_with_env(cmd, Some("claude_code"), None);
        assert_eq!(out, cmd);
    }

    #[test]
    fn test_with_env_passthrough_no_prefix_on_builtin() {
        let out = rewrite_bash_command_with_env("cd /tmp", Some("claude_code"), None);
        assert_eq!(out, "cd /tmp");
    }

    #[test]
    fn test_with_env_no_tool_falls_back_to_plain_rewrite() {
        let out = rewrite_bash_command_with_env("git status", None, None);
        assert_eq!(out, "tok0 git status");
    }

    #[test]
    fn test_with_env_invalid_tool_id_dropped() {
        // Tool id with uppercase / unsafe chars must not be inlined.
        let out = rewrite_bash_command_with_env("git status", Some("Claude Code"), None);
        assert_eq!(out, "tok0 git status");
        let out2 = rewrite_bash_command_with_env("git status", Some("$EVIL"), None);
        assert_eq!(out2, "tok0 git status");
    }

    #[test]
    fn test_with_env_invalid_session_dropped_but_tool_kept() {
        // session_id with unsafe chars is dropped, but the tool var still
        // makes it through.
        let out =
            rewrite_bash_command_with_env("git status", Some("claude_code"), Some("abc; rm -rf /"));
        assert_eq!(out, "TOK0_TOOL=claude_code tok0 git status");
    }

    #[test]
    fn test_with_env_already_tok0_prefixed_no_env_added() {
        // No double-wrap: if cmd is already tok0-prefixed, env not added.
        let out = rewrite_bash_command_with_env("tok0 git status", Some("claude_code"), None);
        assert_eq!(out, "tok0 git status");
    }

    #[test]
    fn test_basic_rewrite() {
        assert_eq!(rewrite_command(&s(&["git", "status"])), "tok0 git status");
    }

    #[test]
    fn test_no_double_rewrite() {
        assert_eq!(
            rewrite_command(&s(&["tok0", "git", "status"])),
            "tok0 git status"
        );
    }

    #[test]
    fn test_empty_args_returns_empty() {
        assert_eq!(rewrite_command(&[]), "");
    }

    #[test]
    fn test_special_chars_preserved() {
        let args = s(&["git", "log", "--format=%H %s"]);
        assert_eq!(rewrite_command(&args), "tok0 git log --format=%H %s");
    }

    #[test]
    fn test_cat_maps_to_read() {
        assert_eq!(
            rewrite_command(&s(&["cat", "src/main.rs"])),
            "tok0 read src/main.rs"
        );
    }

    #[test]
    fn test_head_maps_to_read() {
        assert_eq!(
            rewrite_command(&s(&["head", "-n", "20", "file.txt"])),
            "tok0 read -n 20 file.txt"
        );
    }

    #[test]
    fn test_tail_maps_to_read() {
        assert_eq!(
            rewrite_command(&s(&["tail", "-f", "log.txt"])),
            "tok0 read -f log.txt"
        );
    }

    #[test]
    fn test_rg_maps_to_grep() {
        assert_eq!(
            rewrite_command(&s(&["rg", "pattern", "src/"])),
            "tok0 grep pattern src/"
        );
    }

    #[test]
    fn test_tree_maps_to_ls_tree() {
        assert_eq!(rewrite_command(&s(&["tree"])), "tok0 ls --tree");
        assert_eq!(
            rewrite_command(&s(&["tree", "src/"])),
            "tok0 ls --tree src/"
        );
    }

    #[test]
    fn test_heredoc_passthrough() {
        let args = s(&["bash", "<<EOF"]);
        assert_eq!(rewrite_command(&args), "bash <<EOF");
    }

    #[test]
    fn test_unknown_command_passthrough_with_prefix() {
        assert_eq!(
            rewrite_command(&s(&["cargo", "build", "--release"])),
            "tok0 cargo build --release"
        );
    }

    #[test]
    fn test_command_with_flags() {
        assert_eq!(
            rewrite_command(&s(&["git", "log", "--oneline", "-n", "10"])),
            "tok0 git log --oneline -n 10"
        );
    }

    #[test]
    fn test_cat_no_extra_args() {
        assert_eq!(rewrite_command(&s(&["cat"])), "tok0 read");
    }

    #[test]
    fn test_pr_a_commands_passthrough_unchanged() {
        // These commands have no REWRITE_MAP entry; they should pass through
        // with `tok0` prepended.
        for cmd in [
            "pnpm",
            "bun",
            "bunx",
            "npx",
            "yarn",
            "deno",
            "jest",
            "playwright",
            "cypress",
            "mocha",
            "ava",
            "vite",
            "webpack",
            "esbuild",
            "rollup",
            "parcel",
            "tsup",
            "swc",
            "prettier",
            "next",
            "prisma",
        ] {
            let result = rewrite_command(&[cmd.to_string()]);
            assert!(
                result.starts_with("tok0 "),
                "rewrite for {} should start with `tok0 `, got: {}",
                cmd,
                result
            );
        }
    }

    #[test]
    fn test_standalone_shell_metachars_passthrough() {
        // Shells normally consume these as operators, but if argv
        // contains one as a standalone token (eval-magic, weird
        // quoting, or a misbehaving adapter), the rewriter must not
        // wrap the whole join in `tok0 ` — that would change the
        // shell's parse of the recombined string.
        for op in ["&&", "||", ";", "|", ">", "<", "&"] {
            let args = s(&["echo", "a", op, "echo", "b"]);
            let out = rewrite_command(&args);
            assert_eq!(
                out,
                args.join(" "),
                "metachar {op:?} must trigger passthrough, got: {out}"
            );
        }
    }

    #[test]
    fn test_argv_builtins_and_tty_needers_passthrough() {
        // Mirrors the runtime External-arm + JSON-stdin guards.
        for cmd in ["cd", "export", "FOO=bar", "vim", "sudo", "htop"] {
            let args = s(&[cmd, "args"]);
            let out = rewrite_command(&args);
            assert_eq!(out, args.join(" "), "{cmd} should pass through, got: {out}");
        }
    }

    #[test]
    fn test_double_quoted_args_preserved() {
        // Shell already split these — quotes are gone by the time we see args
        let args = s(&["git", "commit", "-m", "feat: add bridge"]);
        assert_eq!(
            rewrite_command(&args),
            "tok0 git commit -m feat: add bridge"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Property tests (proptest) — invariant checks across random arg vectors.
    // ═══════════════════════════════════════════════════════════════════════
    use proptest::prelude::*;

    /// A single arg token — avoids whitespace (which shells already split on)
    /// and restricts to a non-trivial but finite character set. Strings can
    /// still contain shell metacharacters so we catch encoding bugs.
    fn arg_strategy() -> impl Strategy<Value = String> {
        // Printable ASCII without whitespace, plus common unicode letters.
        proptest::string::string_regex(r"[a-zA-Z0-9_\-./:$&|;()*?!\{\}\[\]=@#%^\+]{1,20}")
            .expect("valid regex")
    }

    fn args_vec() -> impl Strategy<Value = Vec<String>> {
        proptest::collection::vec(arg_strategy(), 1..8)
    }

    /// True if `rewrite_command` is allowed to pass `args` through
    /// unchanged. Mirrors the early-exit cases in the implementation
    /// so property tests can filter them out.
    fn is_passthrough_case(args: &[String]) -> bool {
        if args.is_empty() || args[0] == "tok0" {
            return true;
        }
        if args.iter().any(|a| a.contains("<<")) {
            return true;
        }
        if args
            .iter()
            .any(|a| matches!(a.as_str(), "&&" | "||" | ";" | "|" | ">" | "<" | "&"))
        {
            return true;
        }
        let first = args[0].as_str();
        crate::engine::guards::is_shell_builtin(first)
            || crate::engine::guards::is_var_assignment(first)
            || crate::engine::guards::needs_tty(first)
    }

    proptest! {
        /// Any input that doesn't fall into a passthrough case must
        /// produce output starting with exactly one "tok0 " prefix —
        /// never zero, never two.
        #[test]
        fn prop_output_always_starts_with_tok0_for_non_tok0(args in args_vec()) {
            prop_assume!(!is_passthrough_case(&args));

            let out = rewrite_command(&args);
            prop_assert!(
                out.starts_with("tok0 "),
                "output should start with 'tok0 ', got: {:?}",
                out
            );
            // No double-wrap under any circumstance.
            prop_assert!(
                !out.starts_with("tok0 tok0 "),
                "output should never have doubled tok0 prefix: {:?}",
                out
            );
        }

        /// Heredoc passthrough must NEVER add a tok0 prefix — shell heredoc
        /// parsing would break otherwise.
        #[test]
        fn prop_heredoc_is_passthrough(args in args_vec()) {
            // Inject `<<EOF` into a random arg to trigger the passthrough.
            let mut args = args;
            args.push("<<EOF".to_string());
            let out = rewrite_command(&args);
            prop_assert!(
                !out.starts_with("tok0 "),
                "heredoc input must not be wrapped, got: {:?}",
                out
            );
            prop_assert_eq!(out, args.join(" "));
        }

        /// Idempotency: running rewrite on already-prefixed args is a no-op
        /// after the first pass (no accidental double-wrap when a hook fires
        /// twice or a user manually prefixes).
        #[test]
        fn prop_idempotent_under_tok0_prefix(args in args_vec()) {
            prop_assume!(!args.iter().any(|a| a.contains("<<")));
            let first = rewrite_command(&args);
            // Feed the first-pass output back in as tokenized args.
            let second_args: Vec<String> =
                first.split_whitespace().map(String::from).collect();
            let second = rewrite_command(&second_args);
            prop_assert_eq!(first, second);
        }

        /// Output never drops any of the input tokens (modulo REWRITE_MAP
        /// substitutions, which only affect args[0]). Args[1..] always
        /// appear verbatim in the output.
        #[test]
        fn prop_tail_args_preserved(args in args_vec()) {
            prop_assume!(!is_passthrough_case(&args));

            let out = rewrite_command(&args);
            for tail_arg in &args[1..] {
                prop_assert!(
                    out.contains(tail_arg.as_str()),
                    "tail arg {:?} missing from output {:?}",
                    tail_arg,
                    out
                );
            }
        }

        /// Never panics on arbitrary bytes. This is the fuzzing-lite guarantee.
        #[test]
        fn prop_never_panics(args in proptest::collection::vec(any::<String>(), 0..10)) {
            let _ = rewrite_command(&args);
        }
    }
}
